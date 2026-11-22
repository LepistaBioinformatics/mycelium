#!/usr/bin/env bash
#
# Instance federation, phases F3 (resolution), F4 (degradation) and F5
# (revocation): end to end.
#
# Two postgres-only gateways, A (issuer) and B (consumer), declared to each
# other and active, B accepting A's assertions (`--may-assert`). user-01 has
# a guest role on an account of a tenant of A, and an active identity link
# between B and A, seeded in both databases (making the link through the
# pages is federation-link-e2e.sh). Proves, over HTTP:
#
#   1. `beginners.federation.resolveGrants` refuses anonymous calls, asks A
#      and returns user-01's grant on A, verified, stored and audited;
#   2. the next call is served from the store, without asking A; once the
#      stored assertion is older than assertionRefreshSecs, A is asked again;
#   3. a reference A has no active link for is answered "unknown";
#   4. an assertion naming a tenant id that exists on B is rejected, not
#      stored, and audited;
#   5. the kill-switch on A stops its answers;
#   6. with user-01's account deactivated on B, no grant and no call: a local
#      denial wins;
#   7. user-01's grant revoked on A pushes a tombstone: B drops the stored
#      assertion within seconds, and the next answer carries no grant; a
#      subject A denies (account inactive there) is answered "unknown";
#   8. with B down during the push, A keeps retrying; B drops its stored
#      assertions when it starts and receives the tombstone after;
#   9. with A stopped, an unexpired stored assertion is served as stale, and
#      an expired one contributes nothing: the result is partial, never an
#      error.
#
# Emails are never delivered: the hosted login codes are read from the
# session rows. Everything binds to 127.0.0.1 and lives in a temp dir;
# secrets are generated per run and never printed. Exits 0 when every check
# passes.
#
# Usage:
#   scripts/federation-resolution-e2e.sh [--api-bin <myc-api postgres-only+federation>]
#       [--cli-bin <myc-cli>]

set -euo pipefail
umask 077

API_BIN=""
CLI_BIN=""
SIGNER_BIN=""

while [ $# -gt 0 ]; do
    case "$1" in
        --api-bin) API_BIN="$2"; shift 2 ;;
        --cli-bin) CLI_BIN="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

NEEDS_SIGNER=0
APP_REDIRECT="http://127.0.0.1:9/callback"
EXTRA_CONFIG="[auth.hostedLogin.define]
clients = [{ clientId = \"e2e-app\", redirectUris = [\"$APP_REDIRECT\"] }]"
# shellcheck source=scripts/federation-e2e-common.sh
. "$(dirname "${BASH_SOURCE[0]}")/federation-e2e-common.sh"
# shellcheck source=scripts/federation-e2e-browser.sh
. "$(dirname "${BASH_SOURCE[0]}")/federation-e2e-browser.sh"

# ? ---------------------------------------------------------------------------
# ? Resolution helpers
# ? ---------------------------------------------------------------------------

# subject_ref: a fresh opaque reference, shaped like the ones links mint
subject_ref() { openssl rand 32 | basenc --base64url | tr -d '='; }

user_id() { sql "$1" "select id from public.user where email = '$2'"; }

# seed_grant <database> <email> [suffix]: a reader role on an account of a
# new tenant (names suffixed when seeded again); prints the tenant id
seed_grant() {
    local suffix="${3:-}"
    local tenant account role guest
    read -r tenant account role guest < <(python3 -c 'import uuid; print(*(uuid.uuid4() for _ in range(4)))')
    sql "$1" "insert into tenant (id, name) values ('$tenant', 'tenant-01$suffix');
              insert into account (id, name, slug, tenant_id) values ('$account', 'account-01', 'account-01', '$tenant');
              insert into guest_role (id, name, slug, permission) values ('$role', 'reader$suffix', 'reader$suffix', 0);
              insert into guest_user (id, email, guest_role_id, was_verified) values ('$guest', '$2', '$role', true);
              insert into guest_user_on_account (guest_user_id, account_id) values ('$guest', '$account')" >/dev/null
    printf '%s' "$tenant"
}

# seed_link <database> <user id> <email> <peer instance id> <local ref> <peer ref>
seed_link() {
    sql "$1" "insert into federation_identity_link
                (local_user_id, local_email, peer_id, peer_subject_ref, local_subject_ref, status, established_at)
              select '$2', '$3', id, '$6', '$5', 'active', now() from federation_peer where instance_id = '$4'" >/dev/null
}

resolve() { rpc "$BASE_B" "$1" beginners.federation.resolveGrants '{}'; }

# age_assertions <seconds>: make the stored assertions that old
age_assertions() { sql fed_b "update federation_assertion_cache set iat = now() - interval '$1 seconds'" >/dev/null; }

# account_active <true|false>: switch user-01's account on B, and drop the
# cached profiles so the next call reads it
account_active() {
    sql fed_b "update account set is_active = $1
               where id = (select account_id from public.user where email = 'user-01@example.com');
               delete from kv_artifact" >/dev/null
}

# sql_within <expected> <database> <query>: the query's result once it is
# <expected>, or the last one seen after 60 s (a retry backs off)
sql_within() {
    local value=""
    for _ in $(seq 1 120); do
        value="$(sql "$2" "$3")"
        [ "$value" = "$1" ] && break
        sleep 0.5
    done
    printf '%s' "$value"
}

# make_staff <database> <email>: the email's account becomes a staff account,
# so it may revoke any guest (the profile cache is dropped)
make_staff() {
    sql "$1" "update account set account_type = '\"staff\"'
              where id = (select account_id from public.user where email = '$2');
              delete from kv_artifact" >/dev/null
}

# revoke_grant_on_a: staff user-01 revokes their own reader grant on A's
# tenant (a FED-22 trigger)
revoke_grant_on_a() {
    local account role
    account="$(sql fed_a "select id from account where tenant_id = '$TENANT_A'")"
    role="$(sql fed_a "select g.guest_role_id from guest_user g join guest_user_on_account o on o.guest_user_id = g.id
                       where o.account_id = '$account'")"
    rpc "$BASE_A" "$TOKEN_A" subscriptionsManager.guests.revokeUserGuestToSubscriptionAccount \
        "{\"tenantId\":\"$TENANT_A\",\"accountId\":\"$account\",\"roleId\":\"$role\",\"email\":\"user-01@example.com\"}" >/dev/null
}

forget_assertions() { sql fed_b "delete from federation_assertion_cache; delete from federation_namespace_claim" >/dev/null; }

# audited <count>: wait for the audit dispatcher to write that many rows
audited() {
    local count=""
    for _ in $(seq 1 40); do
        count="$(sql fed_b "select count(*) from resource_audit_log where resource_type = 'federation_resolution'")"
        [ "$count" = "$1" ] && break
        sleep 0.5
    done
    printf '%s' "$count"
}

# ? ---------------------------------------------------------------------------
# ? Run
# ? ---------------------------------------------------------------------------

build_binaries
log "starting Postgres"
start_postgres
DB_A="$(create_database fed_a)"
DB_B="$(create_database fed_b)"

export HMAC_A="$(openssl rand -hex 32)" HMAC_B="$(openssl rand -hex 32)"
read -r PORT_A PORT_B < <(free_ports 2)
BASE_A="http://127.0.0.1:$PORT_A"
BASE_B="http://127.0.0.1:$PORT_B"

log "starting gateways A and B, declared to each other"
start_gateway A "$PORT_A" "$BASE_A" "$DB_A" "$HMAC_A"
start_gateway B "$PORT_B" "$BASE_B" "$DB_B" "$HMAC_B"
sql fed_a "insert into instance_settings (key, value) values ('federation.enabled', 'true')"
sql fed_b "insert into instance_settings (key, value) values ('federation.enabled', 'true')"
ID_A="$(curl "$BASE_A/.well-known/mycelium-instance" | json_field instanceId)"
ID_B="$(curl "$BASE_B/.well-known/mycelium-instance" | json_field instanceId)"
DATABASE_URL="$DB_A" "$CLI_BIN" federation peer add --base-url "$BASE_B" \
    --instance-id "$ID_B" >"$WORKDIR/cli-a.log" 2>&1 || fail "A could not declare B"
DATABASE_URL="$DB_B" "$CLI_BIN" federation peer add --base-url "$BASE_A" --display-name "Instance A" \
    --instance-id "$ID_A" --may-assert >"$WORKDIR/cli-b.log" 2>&1 || fail "B could not declare A"
wait_active fed_a "$ID_B" "B is active on A"
wait_active fed_b "$ID_A" "A is active on B"
expect "profile.v1 is negotiated on B" profile.v1 \
    "$(sql fed_b "select negotiated_kinds->>0 from federation_peer where instance_id = '$ID_A'")"

log "seeding user-01's grant on A and the link between A and B"
TOKEN_B="$(account_token "$BASE_B" fed_b user-01@example.com)"
TOKEN_A="$(account_token "$BASE_A" fed_a user-01@example.com)"
ACCOUNT_A="$(sql fed_a "select account_id from public.user where email = 'user-01@example.com'")"
make_staff fed_a user-01@example.com
TENANT_A="$(seed_grant fed_a user-01@example.com)"
REF_ON_A="$(subject_ref)"
REF_ON_B="$(subject_ref)"
seed_link fed_a "$(user_id fed_a user-01@example.com)" user-01@example.com "$ID_B" "$REF_ON_A" "$REF_ON_B"
seed_link fed_b "$(user_id fed_b user-01@example.com)" user-01@example.com "$ID_A" "$REF_ON_B" "$REF_ON_A"

log "1. B asks A for user-01's grants"
expect "an anonymous call is refused" None "$(rpc "$BASE_B" "" beginners.federation.resolveGrants '{}' | field result)"
FIRST="$(resolve "$TOKEN_B")"
expect "A answered" answered "$(field result.peers.0.resolution.outcome <<<"$FIRST")"
expect "the grant names A's tenant" "$TENANT_A" "$(field result.peers.0.grants.0.tenantId <<<"$FIRST")"
expect "the role travels" reader "$(field result.peers.0.grants.0.role <<<"$FIRST")"
expect "the result is complete" False "$(field result.partial <<<"$FIRST")"
expect "the assertion is stored" 1 "$(sql fed_b "select count(*) from federation_assertion_cache")"
expect "A's tenant is claimed for A" 1 \
    "$(sql fed_b "select count(*) from federation_namespace_claim c join federation_peer p on p.id = c.peer_id
                  where c.foreign_id = '$TENANT_A' and p.instance_id = '$ID_A'")"
expect "the call is audited" 1 "$(audited 1)"

log "2. the next call reads the store, until the assertion is due"
expect "served from the store" stored "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
expect "no new audit row" 1 "$(audited 1)"
age_assertions 40
expect "a due assertion is refreshed" answered "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
expect "the refresh is audited" 2 "$(audited 2)"

log "3. a reference A does not know"
forget_assertions
sql fed_b "update federation_identity_link set peer_subject_ref = '$(subject_ref)'" >/dev/null
expect "A answers unknown" unknown "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
sql fed_b "update federation_identity_link set peer_subject_ref = '$REF_ON_A'" >/dev/null

log "4. an id that exists on B is rejected"
forget_assertions
sql fed_b "insert into tenant (id, name) values ('$TENANT_A', 'tenant-on-b')" >/dev/null
COLLIDING="$(resolve "$TOKEN_B")"
expect "the assertion is rejected" rejected "$(field result.peers.0.resolution.outcome <<<"$COLLIDING")"
expect "the result is partial" True "$(field result.partial <<<"$COLLIDING")"
expect "nothing is stored" 0 "$(sql fed_b "select count(*) from federation_assertion_cache")"
expect "the rejection is audited" 4 "$(audited 4)"
sql fed_b "delete from tenant where id = '$TENANT_A'" >/dev/null

log "5. the kill-switch on A stops its answers"
forget_assertions
sql fed_a "update instance_settings set value = 'false' where key = 'federation.enabled'"
expect "A answers nothing once switched off" unreachable "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
sql fed_a "update instance_settings set value = 'true' where key = 'federation.enabled'"

log "6. a local denial wins"
account_active false
DENIED="$(resolve "$TOKEN_B")"
expect "the account is denied" "the account is inactive" "$(field result.denied <<<"$DENIED")"
expect "no peer contributes" "[]" "$(field result.peers <<<"$DENIED")"
expect "A is not asked" 5 "$(audited 5)"
account_active true

log "7. a revocation on A reaches B within seconds"
expect "A answers" answered "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
revoke_grant_on_a
expect "the grant is revoked on A" 0 "$(sql fed_a "select count(*) from licensed_resources where gu_email = 'user-01@example.com' and tenant_id = '$TENANT_A'")"
expect "B drops the stored assertion" 0 "$(sql_within 0 fed_b "select count(*) from federation_assertion_cache")"
expect "B records the tombstone" grantsChanged "$(sql fed_b "select reason from federation_tombstone")"
expect "A's outbox is drained" 0 "$(sql_within 0 fed_a "select count(*) from federation_outbox")"
sleep 1.1
REVOKED="$(resolve "$TOKEN_B")"
expect "A answers again" answered "$(field result.peers.0.resolution.outcome <<<"$REVOKED")"
expect "without the revoked grant" "[]" "$(field result.peers.0.grants <<<"$REVOKED")"
TENANT_A="$(seed_grant fed_a user-01@example.com -2)"
sql fed_a "update account set is_active = false where id = '$ACCOUNT_A'" >/dev/null
age_assertions 40
expect "A answers unknown for a subject it denies" unknown "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
sql fed_a "update account set is_active = true where id = '$ACCOUNT_A'" >/dev/null
expect "A answers once the subject stands again" answered "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"

log "8. B down during the push receives it after restarting"
RECEIVED="$(sql fed_b "select received_at from federation_tombstone")"
kill "${PIDS[1]}"
wait "${PIDS[1]}" 2>/dev/null || true
revoke_grant_on_a
expect "A keeps the tombstone to retry" 1 \
    "$(sql_within 1 fed_a "select count(*) from federation_outbox where attempts > 0 and status = 'pending'")"
start_gateway B "$PORT_B" "$BASE_B" "$DB_B" "$HMAC_B"
expect "B serves nothing stored before the restart" 0 "$(sql_within 0 fed_b "select count(*) from federation_assertion_cache")"
expect "the retry reaches B" 0 "$(sql_within 0 fed_a "select count(*) from federation_outbox")"
expect "B records the newer tombstone" t \
    "$(sql fed_b "select received_at > '$RECEIVED' from federation_tombstone")"
TENANT_A="$(seed_grant fed_a user-01@example.com -3)"
TOKEN_B="$(account_token "$BASE_B" fed_b user-01@example.com)"
sleep 1.1

log "9. with A stopped, stale within the lifetime and nothing after it"
forget_assertions
expect "A answers while up" answered "$(resolve "$TOKEN_B" | field result.peers.0.resolution.outcome)"
age_assertions 40
kill "${PIDS[0]}"
wait "${PIDS[0]}" 2>/dev/null || true
STALE="$(resolve "$TOKEN_B")"
expect "the stored assertion is served stale" stale "$(field result.peers.0.resolution.outcome <<<"$STALE")"
expect "its grant is still served" "$TENANT_A" "$(field result.peers.0.grants.0.tenantId <<<"$STALE")"
expect "the result is stale" True "$(field result.stale <<<"$STALE")"
expect "but not partial" False "$(field result.partial <<<"$STALE")"
sql fed_b "update federation_assertion_cache set exp = now() - interval '1 second'" >/dev/null
EXPIRED="$(resolve "$TOKEN_B")"
expect "an expired assertion is not served" unreachable "$(field result.peers.0.resolution.outcome <<<"$EXPIRED")"
expect "no grant from A" "[]" "$(field result.peers.0.grants <<<"$EXPIRED")"
expect "the result is partial, not an error" True "$(field result.partial <<<"$EXPIRED")"

if grep -q -e "$HMAC_A" -e "$HMAC_B" "$WORKDIR"/*.log; then
    fail "a secret appears in the logs"
fi
pass "no secret appears in any log"

printf '\nAll %s federation resolution checks passed.\n' "$PASSED"
