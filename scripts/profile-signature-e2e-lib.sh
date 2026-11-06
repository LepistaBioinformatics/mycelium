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

# Go (adapters/nethttp) and Rust (GatewayProfileData) consumers, each with
# its own audience. Reads jwks_env from launch_consumers.
start_compiled_consumers() {
    env "${jwks_env[@]}" MYC_PROFILE_SIGNATURE_AUDIENCE=go-svc \
        "$GO_BIN" "$GO_PORT" >"$WORKDIR/go.log" 2>&1 &
    GO_PID=$!
    env "${jwks_env[@]}" MYC_PROFILE_SIGNATURE_AUDIENCE=rust-svc \
        "$RUST_BIN" "$RUST_PORT" >"$WORKDIR/rust.log" 2>&1 &
    RUST_PID=$!
}

# The kids of the key set the gateway publishes, space separated.
published_kids() {
    curl -fsS "http://127.0.0.1:$API_PORT$JWKS_URL_PATH" | "$PYTHON_BIN" -c \
        'import json,sys; print(" ".join(k["kid"] for k in json.load(sys.stdin)["keys"]))'
}

# expect_published_kids <kids>: the JWKS lists exactly these key versions.
expect_published_kids() {
    local actual
    actual="$(published_kids)" || fail "the gateway does not publish its key set"
    [ "$actual" = "$1" ] || fail "published kids: expected '$1', got '$actual'"
    pass "gateway publishes kids [$1] at $JWKS_URL_PATH"
}

# Verifying consumers start while the gateway publishes no keys, and a
# well-formed signed pair (kid 1) is refused: the failed key download fails
# closed. That attempt also arms the consumers' first-download retry.
expect_consumers_start_without_keys() {
    status_of "http://127.0.0.1:$API_PORT$JWKS_URL_PATH" | grep -qx 404 \
        || fail "the gateway still publishes keys without signing"
    start_consumers jwks
    pass "verifying consumers start while the gateway publishes no keys"
    expect_direct "$PY_PORT" 403 "FastAPI fails closed without the key set" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
    expect_direct "$EXPRESS_PORT" 403 "Express fails closed without the key set" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
    expect_direct "$FASTIFY_PORT" 403 "Fastify fails closed without the key set" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
    expect_direct "$GO_PORT" 403 "Go fails closed without the key set" "$GO_PAIR_PROFILE" "$GO_PAIR_SIGNATURE"
    expect_direct "$RUST_PORT" 403 "Rust fails closed without the key set" "$RUST_PAIR_PROFILE" "$RUST_PAIR_SIGNATURE"
    remember_consumer_pids
}

# Once the gateway publishes its keys, every consumer accepts signed traffic
# within the first-download retry (5 s), without being restarted.
expect_consumers_recover_key_set() {
    local service status
    for service in py-svc express-svc fastify-svc go-svc rust-svc; do
        SERVICE="$service"
        for _ in $(seq 1 40); do
            status="$(whoami_via_gateway)"
            [ "$status" = 200 ] && break
            sleep 0.25
        done
        [ "$status" = 200 ] || fail "$service did not pick up the key set within 10 s (got $status)"
        pass "$service picks up the published key set without a restart -> 200"
    done
    expect_consumers_not_restarted "key set recovery"
}

# Signed profiles reached consumers without MYCELIUM_JWKS_URL: each SDK logs
# the warning once per process. (The Rust test consumer installs no tracing
# subscriber, so its warning is not observable here.)
expect_single_unverified_warning() {
    local name count
    for name in py express fastify go; do
        count="$(grep -c "MYCELIUM_JWKS_URL" "$WORKDIR/$name.log" || true)"
        [ "$count" = 1 ] || fail "$name logged the unverified-signature warning $count times"
        pass "$name warns once that MYCELIUM_JWKS_URL is not set"
    done
}

remember_consumer_pids() {
    ROTATION_PIDS="$PY_PID $EXPRESS_PID $FASTIFY_PID $GO_PID $RUST_PID"
}

# The rotation must not restart any consumer process.
expect_consumers_not_restarted() {
    local pid
    [ "$ROTATION_PIDS" = "$PY_PID $EXPRESS_PID $FASTIFY_PID $GO_PID $RUST_PID" ] \
        || fail "consumers were restarted during the $1"
    for pid in $ROTATION_PIDS; do
        kill -0 "$pid" 2>/dev/null || fail "consumer $pid died during the $1"
    done
    pass "$1 completed with the same consumer processes (no restart)"
}

# An unknown kid refetches at most once per 30 s since the last fetch
# attempt (the first download counts): wait out that window once.
wait_refetch_cooldown() {
    local elapsed=$(( $(date +%s) - CONSUMERS_STARTED ))
    if [ "$elapsed" -lt 31 ]; then
        log "waiting $((31 - elapsed)) s for the consumers' refetch cooldown"
        sleep $((31 - elapsed))
    fi
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
