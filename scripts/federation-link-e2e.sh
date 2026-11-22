#!/usr/bin/env bash
#
# Instance federation, phase F2 (identity links): end to end.
#
# Two postgres-only gateways, A (issuer) and B (consumer), declared to each
# other and active. A user signs in on B through B's hosted login, links the
# account to A through the gateway pages, then unlinks it from A. Proves, over
# HTTP:
#
#   1. `beginners.federation.listPeers` / `startLink` refuse anonymous calls
#      and return an initiation URL that opening does not spend, and that
#      works once;
#   2. A refuses a replayed or tampered redirect state before any login;
#   3. after A's hosted login and the confirmation page, both sides hold the
#      link active, each naming the user with the other's opaque reference;
#   4. different emails are refused by default;
#   5. a state from a quarantined consumer is refused;
#   6. the kill-switch closes A's hosted login to the internal linking client;
#   7. unlinking on A revokes B's copy through the signed revocation.
#
# Form POSTs carry the `Origin` a browser sends; each gateway lists its own
# base URL in `api.allowedOrigins`, as the hosted login already requires.
#
# Emails are never delivered (postgres-only requires SMTP; the e2e points it
# nowhere): the emailed hosted login codes are read from the session rows.
#
# Everything binds to 127.0.0.1 and lives in a temp dir; secrets are generated
# per run and never printed. Exits 0 when every check passes.
#
# Usage:
#   scripts/federation-link-e2e.sh [--api-bin <myc-api postgres-only+federation>]
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
# ? Linking helpers
# ? ---------------------------------------------------------------------------

# start_link <token>: the initiation URL of a new link with A
start_link() {
    rpc "$BASE_B" "$1" beginners.federation.startLink "{\"instanceId\":\"$ID_A\"}" |
        field result.initiationUrl
}

# redirect_state <initiation url>: submit B's start page; prints A's URL
redirect_state() {
    curl -H "Origin: $BASE_B" -D "$WORKDIR/start.headers" -o /dev/null \
        --data-urlencode "request=$(query_param "$1" request)" "$BASE_B/federation/link/start"
    location "$WORKDIR/start.headers"
}

link_status() {
    sql "$1" "select l.status from federation_identity_link l join public.user u on u.id = l.local_user_id
              where u.email = '$2' order by l.created desc limit 1"
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
    --instance-id "$ID_A" >"$WORKDIR/cli-b.log" 2>&1 || fail "B could not declare A"
wait_active fed_a "$ID_B" "B is active on A"
wait_active fed_b "$ID_A" "A is active on B"

log "1. a user on B starts a link with A"
expect "an anonymous call does not list peers" None \
    "$(rpc "$BASE_B" "" beginners.federation.listPeers '{}' | field result)"
TOKEN_B1="$(account_token "$BASE_B" fed_b user-01@example.com)"
expect "A is listed by name" "Instance A" \
    "$(rpc "$BASE_B" "$TOKEN_B1" beginners.federation.listPeers '{}' | field result.0.displayName)"
INITIATION="$(start_link "$TOKEN_B1")"
expect "opening the initiation URL" 200 "$(status_of "$INITIATION")"
expect "opening it again (nothing spent)" 200 "$(status_of "$INITIATION")"
AUTHORIZE="$(redirect_state "$INITIATION")"
expect "B sends the browser to A" "$BASE_A/federation/link/authorize" "${AUTHORIZE%%\?*}"
expect "the initiation URL is spent" 400 "$(status_of "$INITIATION")"
expect "submitting it again" 400 \
    "$(status_of -H "Origin: $BASE_B" --data-urlencode "request=$(query_param "$INITIATION" request)" "$BASE_B/federation/link/start")"

log "2. A checks the state before any login"
expect "a tampered state" 400 "$(status_of "${AUTHORIZE%?}x")"
curl -D "$WORKDIR/authorize.headers" -o /dev/null "$AUTHORIZE"
LOGIN="$(location "$WORKDIR/authorize.headers")"
expect "A sends the browser to its hosted login" /_adm/hosted-login/authorize "${LOGIN%%\?*}"
expect "the same state again" 400 "$(status_of "$AUTHORIZE")"

log "3. the user signs in on A and confirms"
CALLBACK="$(hosted_login "$BASE_A" fed_a user-01@example.com "$WORKDIR/jar-a1" "$BASE_A$LOGIN")"
expect "the hosted login returns to A's callback" "$BASE_A/federation/link/callback" "${CALLBACK%%\?*}"
CONFIRM="$(curl "$CALLBACK")"
grep -q "user-01@example.com" <<<"$CONFIRM" || fail "the confirmation page does not name the accounts"
pass "the confirmation page names the accounts"
curl -H "Origin: $BASE_A" -D "$WORKDIR/approve.headers" -o /dev/null \
    --data-urlencode "consent=$(hidden consent <<<"$CONFIRM")" \
    --data-urlencode "csrf_token=$(hidden csrf_token <<<"$CONFIRM")" "$BASE_A/federation/link/approve"
DONE="$(location "$WORKDIR/approve.headers")"
expect "A sends the browser back to B" "$BASE_B/federation/link/done" "$DONE"
expect "B's done page" 200 "$(status_of "$DONE")"
expect "the link is active on A" active "$(link_status fed_a user-01@example.com)"
expect "the link is active on B" active "$(link_status fed_b user-01@example.com)"
expect "A names the user with B's reference" \
    "$(sql fed_b "select local_subject_ref from federation_identity_link where status = 'active'")" \
    "$(sql fed_a "select peer_subject_ref from federation_identity_link where status = 'active'")"
expect "B names the user with A's reference" \
    "$(sql fed_a "select local_subject_ref from federation_identity_link where status = 'active'")" \
    "$(sql fed_b "select peer_subject_ref from federation_identity_link where status = 'active'")"
expect "the user sees the link on B" active \
    "$(rpc "$BASE_B" "$TOKEN_B1" beginners.federation.listLinks '{}' | field result.0.status)"
expect "a second link with A is refused" None \
    "$(rpc "$BASE_B" "$TOKEN_B1" beginners.federation.startLink "{\"instanceId\":\"$ID_A\"}" | field result)"

log "4. different emails are refused"
TOKEN_B2="$(account_token "$BASE_B" fed_b user-02@example.com)"
AUTHORIZE="$(redirect_state "$(start_link "$TOKEN_B2")")"
curl -D "$WORKDIR/authorize.headers" -o /dev/null "$AUTHORIZE"
CALLBACK="$(hosted_login "$BASE_A" fed_a user-03@example.com "$WORKDIR/jar-a2" "$BASE_A$(location "$WORKDIR/authorize.headers")")"
expect "A refuses user-03 for user-02" 400 "$(status_of "$CALLBACK")"

log "5. a quarantined consumer is refused"
AUTHORIZE="$(redirect_state "$(start_link "$TOKEN_B2")")"
sql fed_a "update federation_peer set status = 'quarantined' where instance_id = '$ID_B'"
expect "A refuses the state of quarantined B" 400 "$(status_of "$AUTHORIZE")"
sql fed_a "update federation_peer set status = 'active' where instance_id = '$ID_B'"

log "6. the kill-switch closes the hosted login to the linking client"
LINK_CLIENT_LOGIN="$BASE_A/_adm/hosted-login/authorize?response_type=code&client_id=mycelium-federation-link&redirect_uri=$BASE_A/federation/link/callback&state=x&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256"
expect "the linking client is served while federation is on" 200 "$(status_of "$LINK_CLIENT_LOGIN")"
sql fed_a "update instance_settings set value = 'false' where key = 'federation.enabled'"
expect "the linking client is refused after the kill-switch" 400 "$(status_of "$LINK_CLIENT_LOGIN")"
sql fed_a "update instance_settings set value = 'true' where key = 'federation.enabled'"

log "7. unlinking on A revokes B's copy"
TOKEN_A1="$(account_token "$BASE_A" fed_a user-01@example.com)"
LINK_A="$(rpc "$BASE_A" "$TOKEN_A1" beginners.federation.listLinks '{}' | field result.0.id)"
expect "the user revokes the link on A" revoked \
    "$(rpc "$BASE_A" "$TOKEN_A1" beginners.federation.revokeLink "{\"linkId\":\"$LINK_A\"}" | field result.status)"
expect "the link is revoked on B" revoked "$(link_status fed_b user-01@example.com)"

if grep -q -e "$HMAC_A" -e "$HMAC_B" "$WORKDIR"/*.log; then
    fail "a secret appears in the logs"
fi
pass "no secret appears in any log"

printf '\nAll %s federation link checks passed.\n' "$PASSED"
