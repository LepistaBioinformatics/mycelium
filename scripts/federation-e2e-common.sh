# Shared helpers of the instance federation end-to-end scripts. Sourced, not
# run. The caller sets API_BIN, CLI_BIN and SIGNER_BIN (empty = build) and
# NEEDS_SIGNER (1 to build the peer request signer), and may set
# EXTRA_CONFIG to append TOML to every gateway config.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="$(mktemp -d)"
RUN_ID="$(date -u +%Y%m%dt%H%M%S)-$RANDOM"
CONTAINER="myc-federation-e2e-$RUN_ID"
PIDS=()
PASSED=0

log() { printf '==> %s\n' "$1"; }
pass() { PASSED=$((PASSED + 1)); printf '  PASS %s\n' "$1"; }
fail() {
    printf 'FAIL: %s\n' "$1" >&2
    printf '  logs kept in %s\n' "$WORKDIR" >&2
    KEEP_WORKDIR=1
    exit 1
}
KEEP_WORKDIR=0

cleanup() {
    local pid
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    done
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
    [ "$KEEP_WORKDIR" = 1 ] || rm -rf "$WORKDIR"
}
trap cleanup EXIT

curl() { command curl --connect-timeout 2 --max-time 10 --silent "$@"; }

free_ports() {
    python3 - "$1" <<'PY'
import socket, sys
sockets = [socket.socket() for _ in range(int(sys.argv[1]))]
for sock in sockets: sock.bind(('127.0.0.1', 0))
print(*(sock.getsockname()[1] for sock in sockets))
for sock in sockets: sock.close()
PY
}

json_field() { python3 -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }

# ? ---------------------------------------------------------------------------
# ? Binaries
# ? ---------------------------------------------------------------------------

build_binaries() {
    if [ -z "$API_BIN" ]; then
        log "building myc-api (postgres-only + federation)"
        cargo build -q --manifest-path "$REPO_ROOT/Cargo.toml" -p mycelium-api \
            --no-default-features --features postgres-only,federation
        API_BIN="$REPO_ROOT/target/debug/myc-api"
    fi
    if [ -z "$CLI_BIN" ]; then
        log "building myc-cli"
        cargo build -q --manifest-path "$REPO_ROOT/Cargo.toml" -p mycelium-cli
        CLI_BIN="$REPO_ROOT/target/debug/myc-cli"
    fi
    if [ "${NEEDS_SIGNER:-0}" = 1 ] && [ -z "$SIGNER_BIN" ]; then
        log "building the peer request signer"
        cargo build -q --manifest-path "$REPO_ROOT/Cargo.toml" \
            -p mycelium-http-tools --example sign_peer_request
        SIGNER_BIN="$REPO_ROOT/target/debug/examples/sign_peer_request"
    fi
}

# ? ---------------------------------------------------------------------------
# ? Postgres: one container, one database per instance
# ? ---------------------------------------------------------------------------

psql_as_owner() { docker exec -i "$CONTAINER" psql -U postgres -q -v ON_ERROR_STOP=1 "$@"; }

start_postgres() {
    export PG_PASSWORD="$(openssl rand -hex 20)"
    docker run -d --name "$CONTAINER" -e POSTGRES_PASSWORD="$PG_PASSWORD" \
        -p 127.0.0.1::5432 postgres:16-alpine >"$WORKDIR/postgres.log"
    for _ in $(seq 1 60); do
        docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1 && break
        sleep 0.5
    done
    sleep 1
    PG_PORT="$(docker port "$CONTAINER" 5432/tcp | head -1 | awk -F: '{print $NF}')"
}

# create_database <name>: up.sql into a new database with its own user
create_database() {
    local name="$1" password
    password="$(openssl rand -hex 20)"
    psql_as_owner -v db_name="$name" -v db_user="${name}_user" -v db_role="${name}_role" \
        -v db_password="$password" <"$REPO_ROOT/adapters/diesel_postgres/sql/up.sql" \
        >"$WORKDIR/schema-$name.log" 2>&1
    printf 'postgres://%s_user:%s@127.0.0.1:%s/%s' "$name" "$password" "$PG_PORT" "$name"
}

# sql <database> <statement>: run as the owner, print the bare result
sql() { psql_as_owner -d "$1" -tAc "$2"; }

# ? ---------------------------------------------------------------------------
# ? Gateways
# ? ---------------------------------------------------------------------------

# write_config <instance> <port> <base url>
write_config() {
    local instance="$1" port="$2" base_url="$3"
    cat >"$WORKDIR/config-$instance-$port.toml" <<EOF
vault = "disabled"
[core.accountLifeCycle]
domainName = "Mycelium federation e2e $instance"
domainUrl = "$base_url"
tokenExpiration = 3600
noreplyName = "Mycelium"
noreplyEmail = "noreply@localhost"
supportName = "Mycelium"
supportEmail = "support@localhost"
locale = "en-us"
tokenSecret = { env = "E2E_TOKEN_SECRET" }
hmacPrimaryVersion = 1

[[core.accountLifeCycle.hmacSecrets]]
version = 1
secret = { env = "E2E_HMAC_SECRET" }

[core.webhook]
acceptInvalidCertificates = true
consumeIntervalInSecs = 30
consumeBatchSize = 25
maxAttempts = 5

[core.federation]
displayName = "Instance $instance"
baseUrl = "$base_url"

[diesel]
databaseUrl = { env = "E2E_DATABASE_URL" }

[smtp]
host = "127.0.0.1"
port = 2525
security = "starttls"
username = { env = "E2E_SMTP_USER" }
password = { env = "E2E_SMTP_PASSWORD" }

[queue]
emailQueueName = "emails"
consumeIntervalInSecs = 60

[auth.internal.define]
jwtExpiresIn = 43200
tmpExpiresIn = 300
jwtSecret = { env = "E2E_JWT_SECRET" }

[api]
serviceIp = "127.0.0.1"
servicePort = $port
serviceWorkers = 1
gatewayTimeout = 60
allowedOrigins = ["$base_url"]
tls = "disabled"

[api.logging]
level = "myc_api=info,myc_core=info"
format = "ansi"
target = "stdout"

${EXTRA_CONFIG:-}
EOF
}

# start_gateway <instance> <port> <base url> <database url> <hmac secret>
start_gateway() {
    local instance="$1" port="$2" base_url="$3" database_url="$4" hmac_secret="$5"
    write_config "$instance" "$port" "$base_url"
    env E2E_DATABASE_URL="$database_url" E2E_HMAC_SECRET="$hmac_secret" \
        E2E_TOKEN_SECRET="$(openssl rand -hex 32)" E2E_JWT_SECRET="$(openssl rand -hex 32)" \
        E2E_SMTP_USER=e2e E2E_SMTP_PASSWORD="$(openssl rand -hex 12)" \
        SETTINGS_PATH="$WORKDIR/config-$instance-$port.toml" \
        "$API_BIN" >"$WORKDIR/api-$instance-$port.log" 2>&1 &
    PIDS+=("$!")
    for _ in $(seq 1 120); do
        [ "$(curl -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port/health")" = 200 ] && return 0
        kill -0 "${PIDS[-1]}" 2>/dev/null || fail "gateway $instance:$port exited (see api-$instance-$port.log)"
        sleep 0.5
    done
    fail "gateway $instance:$port did not answer /health"
}

# ? ---------------------------------------------------------------------------
# ? Requests
# ? ---------------------------------------------------------------------------

status_of() { curl -o /dev/null -w '%{http_code}' "$@"; }

# sign <secret env> <iss> <aud> <method> <path> [jti]
sign() {
    local jti_args=()
    [ -z "${6:-}" ] || jti_args=(--jti "$6")
    "$SIGNER_BIN" --secret-env "$1" --version 1 --iss "$2" --aud "$3" \
        --method "$4" --path "$5" "${jti_args[@]}"
}

# signed_post <port> <path> <jws>: status of a signed POST with an empty body
signed_post() {
    status_of -X POST -H "x-mycelium-peer-signature: $3" "http://127.0.0.1:$1$2"
}

# expect <label> <expected> <actual>
expect() {
    [ "$3" = "$2" ] || fail "$1: expected $2, got $3"
    pass "$1 -> $2"
}

peer_status() { sql "$1" "select status from federation_peer where instance_id = '$2'"; }

# wait_active <database> <peer instance id> <label>: the dispatcher round
# (every 30 s) handshakes the declared peer without any request
wait_active() {
    local status=""
    for _ in $(seq 1 80); do
        status="$(peer_status "$1" "$2")"
        [ "$status" = active ] && break
        sleep 0.5
    done
    expect "$3" active "$status"
}
