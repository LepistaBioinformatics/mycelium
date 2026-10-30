#!/usr/bin/env bash
#
# Downstream profile signature E2E (standalone mode).
#
# Real gateway binary -> real FastAPI / Express / Fastify / Go net/http /
# Rust actix-web consumers built on the LOCAL SDK checkouts (never the
# published packages). Proves, over HTTP:
#
#   1. signing on + verification on: Protected route answers 200 with the
#      caller's real account (profile created through the beginner API);
#   2. a forged inbound signature header is stripped and re-signed;
#   3. direct access to a consumer: unsigned / tampered profile -> 403 and the
#      handler never runs; a captured valid pair replayed inside the window
#      -> 200 (documented limitation); replay to another service is rejected
#      when that service configures its audience;
#   4. wrong rollout order (consumers verify, gateway does not sign) -> 403;
#   5. legacy: nothing configured on either side -> 200;
#   6. rotation: gateway primary v2, consumers trusting v1+v2 -> 200; a
#      consumer trusting only v1 -> 403; retired v1 pairs -> 403;
#   7. invalid signing configuration -> gateway refuses to boot, without
#      printing key material.
#
# Everything binds to 127.0.0.1 and lives in a temp dir; keys are generated
# per run. Only processes started here are stopped. Tokens are never printed.
#
# Usage:
#   scripts/profile-signature-e2e.sh --api-bin <myc-api standalone binary> \
#       --python <python with SDK-py[fastapi,signature] installed> \
#       --node <node> --sdk-js-dir <built mycelium-sdk-js checkout> \
#       --go-bin <built test/profile-signature-consumers/go_app> \
#       --rust-bin <built test/profile-signature-consumers/rust_app>

set -euo pipefail
umask 077
MODE=standalone
EVIDENCE_DIR=""
RUN_ID="$(date -u +%Y%m%dt%H%M%S)-$RANDOM"
INFRA_STARTED=0
BENCHMARK=0

API_BIN=""
PYTHON_BIN="python"
NODE_BIN="node"
SDK_JS_DIR=""
GO_BIN=""
RUST_BIN=""

while [ $# -gt 0 ]; do
    case "$1" in
        --api-bin) API_BIN="$2"; shift 2 ;;
        --go-bin) GO_BIN="$2"; shift 2 ;;
        --rust-bin) RUST_BIN="$2"; shift 2 ;;
        --python) PYTHON_BIN="$2"; shift 2 ;;
        --node) NODE_BIN="$2"; shift 2 ;;
        --sdk-js-dir) SDK_JS_DIR="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --evidence-dir) EVIDENCE_DIR="$2"; shift 2 ;;
        --benchmark) BENCHMARK=1; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONSUMERS="$REPO_ROOT/test/profile-signature-consumers"

case "$MODE" in standalone|postgres-only|full) ;; *) echo 'invalid mode' >&2; exit 2 ;; esac
source "$REPO_ROOT/scripts/profile-signature-e2e-lib.sh"
read -r API_PORT PY_PORT EXPRESS_PORT FASTIFY_PORT GO_PORT RUST_PORT < <("$PYTHON_BIN" - <<'PY'
import socket
sockets = [socket.socket() for _ in range(6)]
for sock in sockets: sock.bind(('127.0.0.1', 0))
print(*(sock.getsockname()[1] for sock in sockets))
for sock in sockets: sock.close()
PY
)
EMAIL="psig-e2e@localhost"
export PSIG_RUN_ID="$RUN_ID" PSIG_CONTROL_TOKEN="$(openssl rand -hex 24)"

WORKDIR="$(mktemp -d)"
API_PID=""
PY_PID=""
EXPRESS_PID=""
FASTIFY_PID=""
GO_PID=""
RUST_PID=""
PASSED=0

log() { printf '==> %s\n' "$1"; }
pass() { PASSED=$((PASSED + 1)); record_result "$1"; printf '  PASS %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

stop_pid() {
    local pid="$1"
    [ -n "$pid" ] || return 0
    local winpid=""
    [ -r "/proc/$pid/winpid" ] && winpid="$(cat "/proc/$pid/winpid")"
    kill "$pid" 2>/dev/null || true
    for _ in $(seq 1 20); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
    kill -9 "$pid" 2>/dev/null || true
    [ -n "$winpid" ] && taskkill //F //T //PID "$winpid" >/dev/null 2>&1 || true
    wait "$pid" 2>/dev/null || true
}

stop_consumers() {
    stop_pid "$PY_PID"; stop_pid "$EXPRESS_PID"; stop_pid "$FASTIFY_PID"
    stop_pid "$GO_PID"; stop_pid "$RUST_PID"
    PY_PID=""; EXPRESS_PID=""; FASTIFY_PID=""; GO_PID=""; RUST_PID=""
}

cleanup() {
    local result=$?
    trap - EXIT
    set +e
    if [ -n "$EVIDENCE_DIR" ]; then
        "$PYTHON_BIN" "$CONSUMERS/collect_evidence.py" "$WORKDIR" "$EVIDENCE_DIR" "$MODE" "$RUN_ID" "$result" "$BENCHMARK"
    fi
    stop_pid "$API_PID"
    stop_consumers
    if [ "$INFRA_STARTED" = 1 ]; then
        compose down --volumes --remove-orphans --timeout 5 >"$WORKDIR/cleanup.log" 2>&1 || result=1
    fi
    if [ -n "$EVIDENCE_DIR" ]; then
        "$PYTHON_BIN" "$CONSUMERS/collect_evidence.py" "$WORKDIR" "$EVIDENCE_DIR" "$MODE" "$RUN_ID" "$result" "$BENCHMARK"
    fi
    [ -n "${KEEP_WORKDIR:-}" ] || rm -rf "$WORKDIR"
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

[ -n "$API_BIN" ] && [ -x "$API_BIN" ] || fail "--api-bin must point to the selected mode myc-api binary"
[ -n "$SDK_JS_DIR" ] && [ -f "$SDK_JS_DIR/dist/express/index.js" ] || fail "--sdk-js-dir must be a built mycelium-sdk-js checkout"
# Node's createRequire (javascript_app.mjs) only accepts absolute paths.
SDK_JS_DIR="$(cd "$SDK_JS_DIR" && pwd)"
[ -n "$GO_BIN" ] && [ -x "$GO_BIN" ] || fail "--go-bin must point to the built Go consumer (go_app)"
[ -n "$RUST_BIN" ] && [ -x "$RUST_BIN" ] || fail "--rust-bin must point to the built Rust consumer (rust_app)"

wait_http() {
    local url="$1" what="$2" log_file="$3"
    for _ in $(seq 1 80); do
        if curl -fsS "$url" >"$WORKDIR/readiness.json" 2>/dev/null; then
            if [ "$what" = gateway ]; then
                kill -0 "$API_PID" 2>/dev/null && return 0
            elif [ "$what" = Mailpit ]; then
                return 0
            elif "$PYTHON_BIN" -c 'import json,sys; assert json.load(open(sys.argv[1]))["runId"] == sys.argv[2]' "$WORKDIR/readiness.json" "$RUN_ID" 2>/dev/null; then
                return 0
            fi
        fi
        sleep 0.25
    done
    fail "$what never became ready (see $log_file)"
}

# Status code of a request; never prints the body.
status_of() { curl -s -o /dev/null -w '%{http_code}' "$@"; }

json_field() { "$PYTHON_BIN" -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }

handler_calls() { curl -fsS "http://127.0.0.1:$1/stats" | json_field whoami; }

# Explicit env resolvers: standalone would otherwise generate and persist
# these secrets in the OS keyring of the machine running the test.
export MYC_E2E_TOKEN_SECRET="$(openssl rand -hex 32)"
export MYC_E2E_HMAC_SECRET="$(openssl rand -hex 32)"
export MYC_E2E_JWT_SECRET="$(openssl rand -hex 32)"

# ? ---------------------------------------------------------------------------
# ? Keys (generated per run, TEST ONLY)
# ? ---------------------------------------------------------------------------

for version in 1 2; do
    openssl genpkey -algorithm ED25519 -out "$WORKDIR/private-v$version.pem" 2>/dev/null
    openssl pkey -in "$WORKDIR/private-v$version.pem" -pubout -out "$WORKDIR/public-v$version.pem" 2>/dev/null
done
log "generated temporary key pairs v1 and v2"

# ? ---------------------------------------------------------------------------
# ? Consumers
# ? ---------------------------------------------------------------------------

# start_consumers <trusted versions, space separated or empty>
# ONLY_V1 lists the consumers pinned to v1 (fastify, go, rust) for rotation.
start_consumers() {
    local versions="$1" key_env=() v1_env=("MYC_PROFILE_PUBLIC_KEY_1=$(cat "$WORKDIR/public-v1.pem")")
    for version in $versions; do
        key_env+=("MYC_PROFILE_PUBLIC_KEY_$version=$(cat "$WORKDIR/public-v$version.pem")")
    done

    env "${key_env[@]}" ENVIRONMENT=production \
        "$PYTHON_BIN" "$CONSUMERS/python_app.py" "$PY_PORT" >"$WORKDIR/py.log" 2>&1 &
    PY_PID=$!
    env "${key_env[@]}" MYC_PROFILE_SIGNATURE_AUDIENCE=express-svc MYC_SDK_JS_DIR="$SDK_JS_DIR" NODE_ENV=production \
        "$NODE_BIN" "$CONSUMERS/javascript_app.mjs" express "$EXPRESS_PORT" >"$WORKDIR/express.log" 2>&1 &
    EXPRESS_PID=$!

    local fastify_env=("${key_env[@]}")
    pinned_to_v1 fastify && fastify_env=("${v1_env[@]}")
    env "${fastify_env[@]}" MYC_SDK_JS_DIR="$SDK_JS_DIR" \
        "$NODE_BIN" "$CONSUMERS/javascript_app.mjs" fastify "$FASTIFY_PORT" >"$WORKDIR/fastify.log" 2>&1 &
    FASTIFY_PID=$!
    start_compiled_consumers

    wait_http "http://127.0.0.1:$PY_PORT/health" "FastAPI consumer" "$WORKDIR/py.log"
    wait_http "http://127.0.0.1:$EXPRESS_PORT/health" "Express consumer" "$WORKDIR/express.log"
    wait_http "http://127.0.0.1:$FASTIFY_PORT/health" "Fastify consumer" "$WORKDIR/fastify.log"
    wait_http "http://127.0.0.1:$GO_PORT/health" "Go consumer" "$WORKDIR/go.log"
    wait_http "http://127.0.0.1:$RUST_PORT/health" "Rust consumer" "$WORKDIR/rust.log"
}

# ? ---------------------------------------------------------------------------
# ? Gateway
# ? ---------------------------------------------------------------------------

# write_config <signing toml block (may be empty)>
write_config() {
    cat >"$WORKDIR/config.toml" <<EOF
vault = "disabled"
[core.accountLifeCycle]
domainName = "Mycelium (profile signature e2e)"
domainUrl = "http://127.0.0.1:$API_PORT"
tokenExpiration = 3600
noreplyName = "Mycelium"
noreplyEmail = "noreply@localhost"
supportName = "Mycelium"
supportEmail = "support@localhost"
locale = "en-us"
tokenSecret = { env = "MYC_E2E_TOKEN_SECRET" }
hmacPrimaryVersion = 1

[[core.accountLifeCycle.hmacSecrets]]
version = 1
secret = { env = "MYC_E2E_HMAC_SECRET" }

$1

[core.webhook]
acceptInvalidCertificates = true
consumeIntervalInSecs = 30
consumeBatchSize = 25
maxAttempts = 5

[sqlite]
path = "$WORKDIR/data/mycelium.db"

[queue]
emailQueueName = "emails"
consumeIntervalInSecs = 1

[auth.internal.define]
jwtExpiresIn = 43200
tmpExpiresIn = 300
jwtSecret = { env = "MYC_E2E_JWT_SECRET" }

[api]
serviceIp = "127.0.0.1"
servicePort = $API_PORT
serviceWorkers = 1
gatewayTimeout = 60
allowedOrigins = ["http://127.0.0.1:$API_PORT"]
tls = "disabled"

[api.logging]
level = "${PSIG_LOG_LEVEL:-myc_api=info,myc_http_tools=info,myc_notifier=info}"
format = "ansi"
target = "stdout"

[api.services]
EOF
    for service in "py-svc:$PY_PORT" "express-svc:$EXPRESS_PORT" "fastify-svc:$FASTIFY_PORT" \
        "go-svc:$GO_PORT" "rust-svc:$RUST_PORT"; do
        local name="${service%%:*}" port="${service##*:}"
        cat >>"$WORKDIR/config.toml" <<EOF

[[$name]]
host = "127.0.0.1:$port"
protocol = "http"
healthCheckPath = "/health"
openapiPath = "/openapi.json"

[[$name.routes]]
path = "/whoami"
methods = ["GET"]
securityGroup = "protected"

[[$name.routes]]
path = "/capture"
methods = ["GET"]
securityGroup = "protected"

[[$name.routes]]
path = "/public"
methods = ["GET"]
securityGroup = "public"

[[$name.routes]]
path = "/authenticated"
methods = ["GET"]
securityGroup = "authenticated"

[[$name.routes]]
path = "/roles"
methods = ["GET"]
securityGroup = { protectedByRoles = [{ slug = "missing-psig-role", permission = "read" }] }
EOF
    done
    append_infra_config
}

signing_block() {
    local primary="$1" versions="$2" block
    block="[core.profileSigning]
primaryKeyVersion = $primary"
    for version in $versions; do
        block="$block

[[core.profileSigning.keys]]
version = $version
privateKey = { env = \"MYC_E2E_PROFILE_PRIVATE_KEY_$version\" }"
    done
    printf '%s' "$block"
}

start_gateway() {
    "$PYTHON_BIN" -c 'import socket,sys; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1); s.bind(("127.0.0.1",int(sys.argv[1]))); s.close()' "$API_PORT"
    SETTINGS_PATH="$WORKDIR/config.toml" \
    MYC_E2E_PROFILE_PRIVATE_KEY_1="$(cat "$WORKDIR/private-v1.pem")" \
    MYC_E2E_PROFILE_PRIVATE_KEY_2="$(cat "$WORKDIR/private-v2.pem")" \
        "$API_BIN" >>"$WORKDIR/api.log" 2>&1 &
    API_PID=$!
    wait_http "http://127.0.0.1:$API_PORT/health" "gateway" "$WORKDIR/api.log"
}

restart_gateway() {
    stop_pid "$API_PID"; API_PID=""
    write_config "$1"
    start_gateway
}

# ? ---------------------------------------------------------------------------
# ? Identity: magic link -> JWT -> account -> profile
# ? ---------------------------------------------------------------------------

authenticate() {
    local base="http://127.0.0.1:$API_PORT/_adm/beginners"
    curl -fsS -X POST "$base/users/magic-link/request" -H 'Content-Type: application/json' \
        -d "{\"email\":\"$EMAIL\"}" >/dev/null || fail "magic-link/request failed"

    # The stub transport prints the email as wrapped plain text (ANSI colors,
    # URL split across indented lines): unwrap it before matching.
    local display_url=""
    for _ in $(seq 1 40); do
        if [ "$MODE" = standalone ]; then
            display_url="$("$PYTHON_BIN" "$CONSUMERS/extract_magic_link.py" "$WORKDIR/api.log" --expected-origin "http://127.0.0.1:$API_PORT")"
        else
            display_url="$("$PYTHON_BIN" "$CONSUMERS/read_test_mail.py" "http://127.0.0.1:$API_PORT" "$EMAIL")"
        fi
        [ -n "$display_url" ] && break
        sleep 0.5
    done
    [ -n "$display_url" ] || fail "no magic-link display URL in the stub transport log"

    local code verify
    code="$(curl -fsS "$display_url" | "$PYTHON_BIN" "$CONSUMERS/extract_magic_link.py" --code)"
    [ -n "$code" ] || fail "no login code on the display page"
    verify="$(curl -fsS -X POST "$base/users/magic-link/verify" -H 'Content-Type: application/json' \
        -d "{\"email\":\"$EMAIL\",\"code\":\"$code\"}")" || fail "magic-link/verify failed"
    JWT="$(printf '%s' "$verify" | json_field token)"
    [ -n "$JWT" ] || fail "no JWT issued"

    local created
    created="$(status_of -X POST "$base/accounts" -H "Authorization: Bearer $JWT" \
        -H 'Content-Type: application/json' -d '{"name":"Profile Signature E2E"}')"
    case "$created" in 200|201) ;; *) fail "account creation returned $created" ;; esac

    ACC_ID="$(curl -fsS "$base/profile" -H "Authorization: Bearer $JWT" | json_field accId)"
    [ -n "$ACC_ID" ] || fail "profile has no accId after account creation"
}

whoami_via_gateway() {
    curl -s -o "$WORKDIR/body.json" -w '%{http_code}' \
        -H "Authorization: Bearer $JWT" "$@" "http://127.0.0.1:$API_PORT/$SERVICE/whoami"
}

expect_gateway_ok() {
    local started="$(date +%s%N)"
    local service="$1" label="$2" status
    SERVICE="$service"; shift 2
    status="$(whoami_via_gateway "$@")"
    [ "$status" = "200" ] || fail "$label: expected 200, got $status"
    [ "$(json_field accId <"$WORKDIR/body.json")" = "$ACC_ID" ] || fail "$label: wrong account"
    SCENARIO_STARTED="$started" OBSERVED_STATUS="$status" EXPECTED_STATUS=200 pass "$label"
}

expect_gateway_status() {
    local started="$(date +%s%N)"
    local service="$1" expected="$2" label="$3" port="$4" before after status
    SERVICE="$service"
    before="$(handler_calls "$port")"
    status="$(whoami_via_gateway)"
    after="$(handler_calls "$port")"
    [ "$status" = "$expected" ] || fail "$label: expected $expected, got $status"
    [ "$expected" = "200" ] || [ "$before" = "$after" ] || fail "$label: handler ran on rejection"
    SCENARIO_STARTED="$started" OBSERVED_STATUS="$status" EXPECTED_STATUS="$expected" HANDLER_BEFORE="$before" HANDLER_AFTER="$after" pass "$label"
}

# expect_direct <port> <expected status> <label> <profile|-> <signature|->
expect_direct() {
    local started="$(date +%s%N)"
    local port="$1" expected="$2" label="$3" profile="$4" signature="$5" args=() before after status
    [ "$profile" = "-" ] || args+=(-H "x-mycelium-profile: $profile")
    [ "$signature" = "-" ] || args+=(-H "x-mycelium-profile-signature: $signature")
    before="$(handler_calls "$port")"
    status="$(status_of "${args[@]}" "http://127.0.0.1:$port/whoami")"
    after="$(handler_calls "$port")"
    [ "$status" = "$expected" ] || fail "$label: expected $expected, got $status"
    [ "$expected" = "200" ] || [ "$before" = "$after" ] || fail "$label: handler ran on rejection"
    SCENARIO_STARTED="$started" OBSERVED_STATUS="$status" EXPECTED_STATUS="$expected" HANDLER_BEFORE="$before" HANDLER_AFTER="$after" pass "$label"
}

# ? ---------------------------------------------------------------------------
# ? Scenarios
# ? ---------------------------------------------------------------------------

log "isolated run $RUN_ID ($MODE)"
start_infra
start_consumers "1"
write_config "$(signing_block 1 "1")"
start_gateway
authenticate
log "authenticated synthetic user with a real account"

if [ "$BENCHMARK" = 1 ]; then
    source "$REPO_ROOT/scripts/profile-signature-benchmark-lib.sh"
    export PSIG_LOG_LEVEL='myc_api=error,myc_http_tools=error,myc_notifier=error'
    run_benchmark_matrix
    exit 0
fi

log "1. gateway signs (v1), consumers verify (v1)"
expect_gateway_ok py-svc "FastAPI via gateway -> 200 with the caller's account"
expect_gateway_ok express-svc "Express via gateway (audience express-svc) -> 200"
expect_gateway_ok fastify-svc "Fastify via gateway -> 200"
expect_gateway_ok go-svc "Go net/http via gateway (audience go-svc) -> 200"
expect_gateway_ok rust-svc "Rust actix-web via gateway (audience rust-svc) -> 200"
expect_gateway_ok py-svc "forged inbound signature is stripped and re-signed -> 200" \
    -H "x-mycelium-profile-signature: forged.by.client"
expect_gateway_ok go-svc "Go: forged inbound signature is stripped and re-signed -> 200" \
    -H "x-mycelium-profile-signature: forged.by.client"
expect_gateway_ok rust-svc "Rust: forged inbound signature is stripped and re-signed -> 200" \
    -H "x-mycelium-profile-signature: forged.by.client"

SERVICE=py-svc
curl -fsS -H "Authorization: Bearer $JWT" -H "x-psig-control: $PSIG_CONTROL_TOKEN" "http://127.0.0.1:$API_PORT/py-svc/capture" >"$WORKDIR/pair.json"
PAIR_PROFILE="$(json_field profile <"$WORKDIR/pair.json")"
PAIR_SIGNATURE="$(json_field signature <"$WORKDIR/pair.json")"
[ -n "$PAIR_SIGNATURE" ] && [ "$PAIR_SIGNATURE" != "None" ] || fail "gateway did not send a signature"
pass "gateway-issued pair captured (aud=py-svc), signature ${#PAIR_SIGNATURE} bytes"
capture_pair go-svc GO
capture_pair rust-svc RUST

JWT="$JWT" ACC_ID="$ACC_ID" API_PORT="$API_PORT" PY_PORT="$PY_PORT" EXPRESS_PORT="$EXPRESS_PORT" FASTIFY_PORT="$FASTIFY_PORT" \
    GO_PORT="$GO_PORT" RUST_PORT="$RUST_PORT" \
    "$PYTHON_BIN" "$CONSUMERS/extended_scenarios.py" "$WORKDIR" "$MODE"

log "2. direct access to consumers (bypassing the gateway)"
expect_direct "$PY_PORT" 403 "FastAPI: forged unsigned profile -> 403" "$PAIR_PROFILE" -
expect_direct "$PY_PORT" 403 "FastAPI: tampered profile with valid signature -> 403" "${PAIR_PROFILE}A" "$PAIR_SIGNATURE"
expect_direct "$PY_PORT" 200 "FastAPI: replay of a valid pair inside the window -> 200 (documented limit)" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
expect_direct "$EXPRESS_PORT" 403 "Express: pair signed for py-svc rejected by audience -> 403" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
expect_direct "$EXPRESS_PORT" 403 "Express: forged unsigned profile -> 403" "$PAIR_PROFILE" -
expect_direct "$FASTIFY_PORT" 403 "Fastify: tampered profile -> 403" "${PAIR_PROFILE}A" "$PAIR_SIGNATURE"
expect_direct "$FASTIFY_PORT" 200 "Fastify without audience accepts a cross-service replay (why audience matters)" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
expect_compiled_direct Go "$GO_PORT" "$GO_PAIR_PROFILE" "$GO_PAIR_SIGNATURE"
expect_compiled_direct Rust "$RUST_PORT" "$RUST_PAIR_PROFILE" "$RUST_PAIR_SIGNATURE"

log "3. wrong rollout order: consumers verify, gateway does not sign"
restart_gateway ""
expect_gateway_status py-svc 403 "FastAPI rejects unsigned gateway traffic" "$PY_PORT"
expect_gateway_status express-svc 403 "Express rejects unsigned gateway traffic" "$EXPRESS_PORT"
expect_gateway_status fastify-svc 403 "Fastify rejects unsigned gateway traffic" "$FASTIFY_PORT"
expect_gateway_status go-svc 403 "Go rejects unsigned gateway traffic" "$GO_PORT"
expect_gateway_status rust-svc 403 "Rust rejects unsigned gateway traffic" "$RUST_PORT"

log "4. legacy: no signing, no verification"
stop_consumers
start_consumers ""
expect_gateway_ok py-svc "FastAPI legacy path -> 200"
expect_gateway_ok express-svc "Express legacy path -> 200"
expect_gateway_ok fastify-svc "Fastify legacy path -> 200"
expect_gateway_ok go-svc "Go legacy path -> 200"
expect_gateway_ok rust-svc "Rust legacy path -> 200"

log "5. rotation: consumers trust v1+v2 (Fastify only v1), gateway primary v2"
stop_consumers
ONLY_V1="fastify" start_consumers "1 2"
restart_gateway "$(signing_block 2 "1 2")"
expect_gateway_ok py-svc "FastAPI accepts kid 2 -> 200"
expect_gateway_ok express-svc "Express accepts kid 2 -> 200"
expect_gateway_status fastify-svc 403 "Fastify trusting only v1 rejects kid 2 -> 403" "$FASTIFY_PORT"
expect_gateway_ok go-svc "Go accepts kid 2 -> 200"
expect_gateway_ok rust-svc "Rust accepts kid 2 -> 200"

log "5a. rotation: Go and Rust trusting only v1, gateway primary v2"
stop_consumers
ONLY_V1="go rust" start_consumers "1 2"
expect_gateway_status go-svc 403 "Go trusting only v1 rejects kid 2 -> 403" "$GO_PORT"
expect_gateway_status rust-svc 403 "Rust trusting only v1 rejects kid 2 -> 403" "$RUST_PORT"

log "5b. retired v1 key is no longer trusted"
stop_consumers
start_consumers "2"
expect_gateway_ok py-svc "FastAPI trusts v2 after retiring v1 -> 200"
expect_gateway_ok express-svc "Express trusts v2 after retiring v1 -> 200"
expect_gateway_ok fastify-svc "Fastify trusts v2 after retiring v1 -> 200"
expect_gateway_ok go-svc "Go trusts v2 after retiring v1 -> 200"
expect_gateway_ok rust-svc "Rust trusts v2 after retiring v1 -> 200"
expect_direct "$PY_PORT" 403 "FastAPI retired v1 pair is refused" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
expect_direct "$FASTIFY_PORT" 403 "Fastify retired v1 pair is refused" "$PAIR_PROFILE" "$PAIR_SIGNATURE"
expect_direct "$GO_PORT" 403 "Go retired v1 pair (aud go-svc) is refused" "$GO_PAIR_PROFILE" "$GO_PAIR_SIGNATURE"
expect_direct "$RUST_PORT" 403 "Rust retired v1 pair (aud rust-svc) is refused" "$RUST_PAIR_PROFILE" "$RUST_PAIR_SIGNATURE"

log "6. invalid signing configuration aborts the boot"
stop_pid "$API_PID"; API_PID=""
write_config "$(signing_block 1 "1")"
: >"$WORKDIR/boot-fail.log"
set +e
SETTINGS_PATH="$WORKDIR/config.toml" MYC_E2E_PROFILE_PRIVATE_KEY_1="-----BEGIN PRIVATE KEY-----
c2VjcmV0LW1hdGVyaWFs
-----END PRIVATE KEY-----" "$API_BIN" >"$WORKDIR/boot-fail.log" 2>&1 &
BOOT_PID=$!
for _ in $(seq 1 240); do
    kill -0 "$BOOT_PID" 2>/dev/null || break
    sleep 0.25
done
if kill -0 "$BOOT_PID" 2>/dev/null; then
    stop_pid "$BOOT_PID"
    fail "gateway kept running with an invalid signing key"
fi
wait "$BOOT_PID"
BOOT_STATUS=$?
set -e
[ "$BOOT_STATUS" -ne 0 ] || fail "gateway did not refuse to boot (status $BOOT_STATUS)"
grep -q "profile_signing_key_invalid: version 1" "$WORKDIR/boot-fail.log" || fail "boot failure does not name the key version"
! grep -q "c2VjcmV0LW1hdGVyaWFs" "$WORKDIR/boot-fail.log" || fail "boot failure leaked key material"
curl -fsS "http://127.0.0.1:$API_PORT/health" >/dev/null 2>&1 && fail "gateway is serving after a failed boot"
pass "boot refused (exit $BOOT_STATUS), sanitized error, nothing listening"

echo
echo "ALL $PASSED CHECKS PASSED ($MODE)"
