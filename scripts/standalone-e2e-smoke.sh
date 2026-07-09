#!/usr/bin/env bash
#
# Standalone Mode E2E smoke test (SM-T30, closes SM-R13 / completes SM-T26).
#
# Boots the real standalone binary (`--no-default-features --features
# standalone`) against an empty temp dir and drives the full zero-config
# onboarding flow end to end:
#
#   health -> magic-link request -> display -> verify (JWT issued)
#          -> add a downstream route (config) -> proxy a request through it
#
# Unlike prior manual verification sessions (see STATE.md), this script is
# repeatable and asserts each step instead of relying on eyeballing logs.
#
# Usage:
#   scripts/standalone-e2e-smoke.sh [path-to-standalone-binary] [path-to-test-svc-binary]
#
# Defaults assume both binaries were already built from the repo root:
#   cargo build -p mycelium-api --no-default-features --features standalone
#   cargo build -p mycelium-api-test-svc --bin myc-api-test-svc

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_BIN="${1:-$REPO_ROOT/target/debug/myc-api}"
TEST_SVC_BIN="${2:-$REPO_ROOT/target/debug/myc-api-test-svc}"

API_PORT=18080
TEST_SVC_PORT=18099
EMAIL="e2e-smoke@localhost"

WORKDIR="$(mktemp -d)"
API_LOG="$WORKDIR/api.log"
SVC_LOG="$WORKDIR/test-svc.log"
API_PID=""
SVC_PID=""

log() { printf '==> %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

cleanup() {
    [ -n "$API_PID" ] && kill "$API_PID" 2>/dev/null || true
    [ -n "$SVC_PID" ] && kill "$SVC_PID" 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT

[ -x "$API_BIN" ] || fail "standalone binary not found/executable at $API_BIN (build it first)"
[ -x "$TEST_SVC_BIN" ] || fail "test service binary not found/executable at $TEST_SVC_BIN (build it first)"

log "workdir: $WORKDIR"

# -----------------------------------------------------------------------
# 1. Downstream test service (reuses the repo's own test/downstream_service
#    -- already proven in the dev docker-compose stack -- instead of a
#    throwaway stub, so the proxied response has a real, known shape).
# -----------------------------------------------------------------------
log "starting downstream test service on :$TEST_SVC_PORT"
SERVICE_PORT="$TEST_SVC_PORT" "$TEST_SVC_BIN" >"$SVC_LOG" 2>&1 &
SVC_PID=$!

for _ in $(seq 1 30); do
    curl -fsS "http://127.0.0.1:$TEST_SVC_PORT/health" >/dev/null 2>&1 && break
    sleep 0.5
done
curl -fsS "http://127.0.0.1:$TEST_SVC_PORT/health" >/dev/null 2>&1 \
    || fail "downstream test service never became healthy (see $SVC_LOG)"
log "downstream test service healthy"

# -----------------------------------------------------------------------
# 2. Standalone config -- includes a downstream route from the start
#    (SM-R13's "a downstream route can be added" is a config-driven
#    operation in this codebase: routes are declared under
#    [api.services]/[[service-name]], not created via a REST call --
#    confirmed by reading `ApiConfig::deserialize_services` and
#    `build_mem_db_module` in `ports/api/src/main.rs`).
# -----------------------------------------------------------------------
cat >"$WORKDIR/config.toml" <<EOF
[core.accountLifeCycle]
domainName = "Mycelium (e2e-smoke)"
domainUrl = "http://127.0.0.1:$API_PORT"
tokenExpiration = 3600
noreplyName = "Mycelium"
noreplyEmail = "noreply@localhost"
supportName = "Mycelium"
supportEmail = "support@localhost"
locale = "en-us"
tokenSecret = "placeholder"
hmacPrimaryVersion = 1

[[core.accountLifeCycle.hmacSecrets]]
version = 1
secret = "placeholder"

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
jwtSecret = "placeholder"

[api]
serviceIp = "0.0.0.0"
servicePort = $API_PORT
serviceWorkers = 1
gatewayTimeout = 60
allowedOrigins = ["http://127.0.0.1:$API_PORT"]
tls = "disabled"

[api.logging]
level = "mycelium_base=info,myc_api=info,myc_config=info,myc_core=info,myc_http_tools=info,actix_web=info,myc_notifier=info,myc_diesel_sqlite=info,myc_moka_cache=info"
format = "ansi"
target = "stdout"

[api.services]

[[stub-service]]
host = "127.0.0.1:$TEST_SVC_PORT"
protocol = "http"
healthCheckPath = "/health"

[[stub-service.routes]]
path = "/public"
methods = ["ALL"]
securityGroup = "public"
EOF

# -----------------------------------------------------------------------
# 3. Boot the standalone binary
# -----------------------------------------------------------------------
log "booting standalone binary on :$API_PORT"
SETTINGS_PATH="$WORKDIR/config.toml" "$API_BIN" >"$API_LOG" 2>&1 &
API_PID=$!

for _ in $(seq 1 60); do
    curl -fsS "http://127.0.0.1:$API_PORT/health" >/dev/null 2>&1 && break
    sleep 0.5
done
curl -fsS "http://127.0.0.1:$API_PORT/health" >/dev/null 2>&1 \
    || fail "standalone gateway never became healthy (see $API_LOG)"
log "standalone gateway healthy"

# -----------------------------------------------------------------------
# 4. Magic-link request -> display -> verify -> JWT
# -----------------------------------------------------------------------
BASE="http://127.0.0.1:$API_PORT/_adm/beginners/users"

log "requesting magic link for $EMAIL"
curl -fsS -X POST "$BASE/magic-link/request" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$EMAIL\"}" >/dev/null \
    || fail "magic-link/request failed"

# The email dispatcher polls on an interval (`[queue] consumeIntervalInSecs`)
# rather than sending synchronously, so the stub transport's log line appears
# some time after the request returns -- poll for it instead of checking once.
# The logged body is the rendered (Tera-escaped) HTML email: `/` and `&` come
# through as `&#x2F;` / `&amp;` inside the `href="..."` attribute, so extract
# from the escaped form and decode before using the URL.
RAW_LINK=""
for _ in $(seq 1 40); do
    RAW_LINK="$(grep -oE 'href="[^"]*magic-link&#x2F;display\?[^"]+"' "$API_LOG" | tail -n1 || true)"
    [ -n "$RAW_LINK" ] && break
    sleep 0.5
done
[ -n "$RAW_LINK" ] || fail "no magic-link display URL found in stub transport log ($API_LOG)"

DISPLAY_URL="$(printf '%s' "$RAW_LINK" \
    | sed -E 's/^href="//; s/"$//; s/&#x2F;/\//g; s/&amp;/\&/g')"
log "extracted display URL from stub transport log"

DISPLAY_HTML="$(curl -fsS "$DISPLAY_URL")" \
    || fail "GET magic-link/display failed"
CODE="$(printf '%s' "$DISPLAY_HTML" | grep -oE '<div class="code">[^<]+</div>' | sed -E 's/<[^>]+>//g' | tr -d '[:space:]')"
[ -n "$CODE" ] || fail "could not extract the 6-digit code from the display page"
log "extracted login code: $CODE"

VERIFY_RESPONSE="$(curl -fsS -X POST "$BASE/magic-link/verify" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$EMAIL\",\"code\":\"$CODE\"}")" \
    || fail "magic-link/verify failed"

JWT="$(printf '%s' "$VERIFY_RESPONSE" | grep -oE '"token":"[^"]+"' | cut -d'"' -f4)"
[ -n "$JWT" ] || fail "no JWT in magic-link/verify response: $VERIFY_RESPONSE"
log "JWT issued (${#JWT} chars) -- SM-R13's 'a JWT can be issued' verified"

# -----------------------------------------------------------------------
# 5. Proxy a request through the downstream route added in step 2
# -----------------------------------------------------------------------
log "proxying a request through the configured downstream route"
PROXY_RESPONSE="$(curl -fsS "http://127.0.0.1:$API_PORT/stub-service/public")" \
    || fail "proxied request through /stub-service/public failed"

printf '%s' "$PROXY_RESPONSE" | grep -q "success" \
    || fail "proxied response did not match the downstream service's expected body: $PROXY_RESPONSE"
log "proxied request succeeded -- SM-R13's 'a downstream route can be added' + proxy verified"

echo
echo "ALL STEPS PASSED: health, JWT issuance, downstream route + proxy -- SM-R13 satisfied end to end."
