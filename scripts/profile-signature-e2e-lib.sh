#!/usr/bin/env bash
# Imported by the E2E runner. No actions at import time.
curl() { command curl --connect-timeout 2 --max-time 10 --globoff "$@"; }

docker_cli() {
    if [ "${PSIG_DOCKER_WSL_ROOT:-0}" = 1 ]; then
        # WSL only: the current distro (WSL_DISTRO_NAME), as root, via interop.
        "${PSIG_WSL_EXE:-wsl.exe}" -d "${PSIG_WSL_DISTRO:-${WSL_DISTRO_NAME:?PSIG_DOCKER_WSL_ROOT=1 requires WSL}}" -u root -- env \
            "PSIG_DB_PASSWORD=${PSIG_DB_PASSWORD:-}" "PSIG_REDIS_PASSWORD=${PSIG_REDIS_PASSWORD:-}" \
            "PSIG_SMTP_USER=${PSIG_SMTP_USER:-}" "PSIG_SMTP_PASSWORD=${PSIG_SMTP_PASSWORD:-}" \
            "PSIG_TLS_DIR=${PSIG_TLS_DIR:-}" docker "$@"
    else
        docker "$@"
    fi
}

compose() {
    docker_cli compose --project-name "$COMPOSE_PROJECT_NAME" \
        --file "$CONSUMERS/compose.yaml" --profile "$MODE" "$@"
}

infra_port() { compose port "$1" "$2" | tr -d '\r' | awk -F: '{print $NF}'; }

start_infra() {
    [ "$MODE" != standalone ] || return 0
    export PSIG_DB_PASSWORD="$(openssl rand -hex 20)"
    export PSIG_REDIS_PASSWORD="$(openssl rand -hex 20)"
    export PSIG_SMTP_USER=psig PSIG_SMTP_PASSWORD="$(openssl rand -hex 20)"
    export PSIG_TLS_DIR="$WORKDIR/tls" COMPOSE_PROJECT_NAME="psig-$RUN_ID"
    mkdir -p "$PSIG_TLS_DIR"
    openssl req -x509 -newkey rsa:2048 -nodes -days 2 -subj /CN=PSIG-Test-CA \
        -addext 'basicConstraints=critical,CA:TRUE' -addext 'keyUsage=critical,keyCertSign,cRLSign' \
        -keyout "$PSIG_TLS_DIR/ca.key" -out "$PSIG_TLS_DIR/ca.pem" >"$WORKDIR/tls.log" 2>&1
    openssl req -newkey rsa:2048 -nodes -subj /CN=localhost \
        -keyout "$PSIG_TLS_DIR/server.key" -out "$PSIG_TLS_DIR/server.csr" >>"$WORKDIR/tls.log" 2>&1
    printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\nkeyUsage=critical,digitalSignature,keyEncipherment\n' >"$PSIG_TLS_DIR/extensions"
    openssl x509 -req -days 2 -in "$PSIG_TLS_DIR/server.csr" -CA "$PSIG_TLS_DIR/ca.pem" \
        -CAkey "$PSIG_TLS_DIR/ca.key" -CAcreateserial -extfile "$PSIG_TLS_DIR/extensions" \
        -out "$PSIG_TLS_DIR/server.pem" >>"$WORKDIR/tls.log" 2>&1
    chmod 644 "$PSIG_TLS_DIR/server.key"
    INFRA_STARTED=1
    compose up -d --wait --wait-timeout 60 >"$WORKDIR/compose.log" 2>&1
    export PSIG_POSTGRES_PORT="$(infra_port postgres 5432)"
    export PSIG_SMTP_PORT="$(infra_port mailpit 1025)"
    export PSIG_MAIL_API="http://127.0.0.1:$(infra_port mailpit 8025)"
    if [ "$MODE" = full ]; then export PSIG_REDIS_PORT="$(infra_port redis 6379)"; fi
    export SSL_CERT_FILE="$PSIG_TLS_DIR/ca.pem"
    wait_http "$PSIG_MAIL_API/readyz" Mailpit "$WORKDIR/compose.log"
    "$PYTHON_BIN" "$CONSUMERS/smtp_probe.py" >"$WORKDIR/smtp-probe.log" 2>&1
    compose exec -T postgres psql -U postgres -v ON_ERROR_STOP=1 \
        -v db_name=psig -v db_user=psig -v db_role=psig_role -v db_password="$PSIG_DB_PASSWORD" \
        <"$REPO_ROOT/adapters/diesel_postgres/sql/up.sql" >"$WORKDIR/schema.log" 2>&1
    export PSIG_DATABASE_URL="postgres://psig:$PSIG_DB_PASSWORD@127.0.0.1:$PSIG_POSTGRES_PORT/psig"
}

# pinned_to_v1 <consumer>: listed in ONLY_V1 (rotation scenarios).
pinned_to_v1() { case " ${ONLY_V1:-} " in *" $1 "*) return 0 ;; esac; return 1; }

# Go (adapters/nethttp) and Rust (GatewayProfileData) consumers, each with
# its own audience. Reads key_env/v1_env from start_consumers.
start_compiled_consumers() {
    local go_env=("${key_env[@]}") rust_env=("${key_env[@]}")
    pinned_to_v1 go && go_env=("${v1_env[@]}")
    pinned_to_v1 rust && rust_env=("${v1_env[@]}")
    env "${go_env[@]}" MYC_PROFILE_SIGNATURE_AUDIENCE=go-svc \
        "$GO_BIN" "$GO_PORT" >"$WORKDIR/go.log" 2>&1 &
    GO_PID=$!
    env "${rust_env[@]}" MYC_PROFILE_SIGNATURE_AUDIENCE=rust-svc \
        "$RUST_BIN" "$RUST_PORT" >"$WORKDIR/rust.log" 2>&1 &
    RUST_PID=$!
}

# capture_pair <service> <variable prefix>: gateway-issued pair for that
# service audience; values stay in shell variables and the private workdir.
capture_pair() {
    local service="$1" prefix="$2" profile signature
    curl -fsS -H "Authorization: Bearer $JWT" -H "x-psig-control: $PSIG_CONTROL_TOKEN" \
        "http://127.0.0.1:$API_PORT/$service/capture" >"$WORKDIR/pair-$service.json"
    profile="$(json_field profile <"$WORKDIR/pair-$service.json")"
    signature="$(json_field signature <"$WORKDIR/pair-$service.json")"
    [ -n "$signature" ] && [ "$signature" != "None" ] || fail "gateway did not sign for $service"
    printf -v "${prefix}_PAIR_PROFILE" '%s' "$profile"
    printf -v "${prefix}_PAIR_SIGNATURE" '%s' "$signature"
    pass "gateway-issued pair captured (aud=$service), signature ${#signature} bytes"
}

# expect_compiled_direct <framework> <port> <own profile> <own signature>
expect_compiled_direct() {
    local name="$1" port="$2" profile="$3" signature="$4"
    expect_direct "$port" 403 "$name: forged unsigned profile -> 403" "$profile" -
    expect_direct "$port" 403 "$name: tampered profile with valid signature -> 403" "${profile}A" "$signature"
    expect_direct "$port" 403 "$name: pair signed for py-svc rejected by audience -> 403" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
    expect_direct "$port" 200 "$name: replay of its own valid pair inside the window -> 200 (documented limit)" "$profile" "$signature"
}

append_infra_config() {
    [ "$MODE" != standalone ] || return 0
    "$PYTHON_BIN" "$CONSUMERS/write_test_config.py" "$WORKDIR/config.toml" "$MODE"
}

record_result() {
    "$PYTHON_BIN" - "$WORKDIR/results.jsonl" "$MODE" "$PASSED" "$1" \
        "${OBSERVED_STATUS:-}" "${EXPECTED_STATUS:-}" "${HANDLER_BEFORE:-}" "${HANDLER_AFTER:-}" "${SCENARIO_STARTED:-}" <<'PY'
import json, sys, time
path, mode, number, label, observed, expected, before, after, started = sys.argv[1:]
with open(path, 'a', encoding='utf-8') as stream:
    stream.write(json.dumps(dict(id=f'original-{int(number):02}', mode=mode, label=label,
        outcome='pass', observedStatus=observed or None, expectedStatus=expected or None,
        handlerBefore=before or None, handlerAfter=after or None,
        durationMs=(time.time_ns()-int(started))/1e6 if started else None))+'\n')
PY
}
