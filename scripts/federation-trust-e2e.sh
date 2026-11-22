#!/usr/bin/env bash
#
# Instance federation, phase F1 (trust between peers): end to end.
#
# Two postgres-only gateways, A and B, with different hmacSecrets, each in its
# own database of one throwaway Postgres container. B runs two pods on the
# same database. Proves, over HTTP:
#
#   1. each instance serves its self-description once federation is on;
#   2. each declares the other with `myc-cli federation peer add`; the
#      federation dispatcher alone brings both peers to `active` within one
#      health round, and a signed handshake request is served;
#   3. a request signed by an undeclared third key, or claiming an undeclared
#      issuer, is refused;
#   4. a signed request accepted by one pod of B is refused as a replay by
#      the other pod (the jti store is shared);
#   5. the role catalog is served to a declared peer only (default mode);
#   6. a quarantined peer is refused on its very next request;
#   7. the kill-switch makes the self-description 404 and refuses every peer
#      request.
#
# Quarantine and the kill-switch are applied with SQL, exactly the rows the
# staff RPC (`federation.peers.quarantine`, `federation.setEnabled`) writes;
# the RPC itself is covered by its unit tests.
#
# Everything binds to 127.0.0.1 and lives in a temp dir; secrets are generated
# per run and never printed. Only processes and containers started here are
# stopped. Exits 0 when every check passes, non-zero on the first failure.
#
# Usage:
#   scripts/federation-trust-e2e.sh [--api-bin <myc-api postgres-only+federation>]
#       [--cli-bin <myc-cli>] [--signer-bin <sign_peer_request example>]
#
# Missing binaries are built with cargo.

set -euo pipefail
umask 077

API_BIN=""
CLI_BIN=""
SIGNER_BIN=""

while [ $# -gt 0 ]; do
    case "$1" in
        --api-bin) API_BIN="$2"; shift 2 ;;
        --cli-bin) CLI_BIN="$2"; shift 2 ;;
        --signer-bin) SIGNER_BIN="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

NEEDS_SIGNER=1
# shellcheck source=scripts/federation-e2e-common.sh
. "$(dirname "${BASH_SOURCE[0]}")/federation-e2e-common.sh"

# ? ---------------------------------------------------------------------------
# ? Run
# ? ---------------------------------------------------------------------------

build_binaries
log "starting Postgres"
start_postgres
DB_A="$(create_database fed_a)"
DB_B="$(create_database fed_b)"

export HMAC_A="$(openssl rand -hex 32)" HMAC_B="$(openssl rand -hex 32)"
export HMAC_THIRD="$(openssl rand -hex 32)"
read -r PORT_A PORT_B1 PORT_B2 < <(free_ports 3)
BASE_A="http://127.0.0.1:$PORT_A"
BASE_B="http://127.0.0.1:$PORT_B1"

log "starting gateway A and two pods of gateway B"
start_gateway A "$PORT_A" "$BASE_A" "$DB_A" "$HMAC_A"
start_gateway B "$PORT_B1" "$BASE_B" "$DB_B" "$HMAC_B"
start_gateway B "$PORT_B2" "$BASE_B" "$DB_B" "$HMAC_B"

log "1. self-description follows the runtime switch"
expect "A self-description while federation is off" 404 \
    "$(status_of "$BASE_A/.well-known/mycelium-instance")"
sql fed_a "insert into instance_settings (key, value) values ('federation.enabled', 'true')"
sql fed_b "insert into instance_settings (key, value) values ('federation.enabled', 'true')"
expect "A self-description once switched on" 200 \
    "$(status_of "$BASE_A/.well-known/mycelium-instance")"
ID_A="$(curl "$BASE_A/.well-known/mycelium-instance" | json_field instanceId)"
ID_B="$(curl "$BASE_B/.well-known/mycelium-instance" | json_field instanceId)"
ID_B2="$(curl "http://127.0.0.1:$PORT_B2/.well-known/mycelium-instance" | json_field instanceId)"
expect "both pods of B announce the same instance id" "$ID_B" "$ID_B2"

log "2. peers declared with the CLI and handshaked"
DATABASE_URL="$DB_A" "$CLI_BIN" federation peer add --base-url "$BASE_B" \
    --instance-id "$ID_B" >"$WORKDIR/cli-a.log" 2>&1 || fail "A could not declare B"
DATABASE_URL="$DB_B" "$CLI_BIN" federation peer add --base-url "$BASE_A" \
    --instance-id "$ID_A" >"$WORKDIR/cli-b.log" 2>&1 || fail "B could not declare A"
expect "B is declared on A" declared "$(peer_status fed_a "$ID_B")"
expect "A declaring B again is a conflict" 1 \
    "$(DATABASE_URL="$DB_A" "$CLI_BIN" federation peer add --base-url "$BASE_B" \
        --instance-id "$ID_B" >/dev/null 2>&1; echo $?)"

wait_active fed_a "$ID_B" "the dispatcher of A brings B to active"
wait_active fed_b "$ID_A" "the dispatcher of B brings A to active"

expect "A asks B for a handshake" 200 \
    "$(signed_post "$PORT_B1" /federation/handshake \
        "$(sign HMAC_A "$ID_A" "$ID_B" POST /federation/handshake)")"
expect "A is active on B" active "$(peer_status fed_b "$ID_A")"
expect "B asks A for a handshake" 200 \
    "$(signed_post "$PORT_A" /federation/handshake \
        "$(sign HMAC_B "$ID_B" "$ID_A" POST /federation/handshake)")"
expect "B is active on A" active "$(peer_status fed_a "$ID_B")"
expect "A stored the key of B" 1 \
    "$(sql fed_a "select count(*) from federation_peer_key k join federation_peer p on p.id = k.peer_id where p.instance_id = '$ID_B'")"

log "3. undeclared callers are refused"
expect "A's id signed by a third key" 401 \
    "$(signed_post "$PORT_B1" /federation/handshake \
        "$(sign HMAC_THIRD "$ID_A" "$ID_B" POST /federation/handshake)")"
expect "an undeclared issuer" 401 \
    "$(signed_post "$PORT_B1" /federation/handshake \
        "$(sign HMAC_THIRD "$(python3 -c 'import uuid; print(uuid.uuid4())')" "$ID_B" POST /federation/handshake)")"
expect "a request signed for another audience" 401 \
    "$(signed_post "$PORT_B1" /federation/handshake \
        "$(sign HMAC_A "$ID_A" "$ID_A" POST /federation/handshake)")"

log "4. a replay is refused by the other pod"
REPLAYED="$(sign HMAC_A "$ID_A" "$ID_B" POST /federation/handshake)"
expect "first use on pod 1 of B" 200 "$(signed_post "$PORT_B1" /federation/handshake "$REPLAYED")"
expect "replay on pod 2 of B" 401 "$(signed_post "$PORT_B2" /federation/handshake "$REPLAYED")"

log "5. the role catalog is served to a declared peer only"
CATALOG=/.well-known/mycelium-instance/roles
expect "catalog without a signature" 401 "$(status_of "$BASE_B$CATALOG")"
expect "catalog signed by A" 200 \
    "$(status_of -H "x-mycelium-peer-signature: $(sign HMAC_A "$ID_A" "$ID_B" GET "$CATALOG")" "$BASE_B$CATALOG")"
expect "B advertises its catalog" "$BASE_B$CATALOG" \
    "$(curl "$BASE_B/.well-known/mycelium-instance" | json_field roleCatalogUrl)"

log "6. quarantine takes effect on the next request"
sql fed_b "update federation_peer set status = 'quarantined' where instance_id = '$ID_A'"
expect "quarantined A asks B for a handshake" 403 \
    "$(signed_post "$PORT_B2" /federation/handshake \
        "$(sign HMAC_A "$ID_A" "$ID_B" POST /federation/handshake)")"
sql fed_b "update federation_peer set status = 'declared' where instance_id = '$ID_A'"
expect "released A is served again" 200 \
    "$(signed_post "$PORT_B2" /federation/handshake \
        "$(sign HMAC_A "$ID_A" "$ID_B" POST /federation/handshake)")"

log "7. the kill-switch stops federation"
sql fed_b "update instance_settings set value = 'false' where key = 'federation.enabled'"
expect "B self-description after the kill-switch" 404 \
    "$(status_of "$BASE_B/.well-known/mycelium-instance")"
expect "signed request after the kill-switch" 404 \
    "$(signed_post "$PORT_B1" /federation/handshake \
        "$(sign HMAC_A "$ID_A" "$ID_B" POST /federation/handshake)")"

if grep -q -e "$HMAC_A" -e "$HMAC_B" -e "$HMAC_THIRD" "$WORKDIR"/*.log; then
    fail "a secret appears in the logs"
fi
pass "no secret appears in any log"

printf '\nAll %s federation trust checks passed.\n' "$PASSED"
