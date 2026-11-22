# Browser and JSON-RPC helpers of the instance federation end-to-end scripts.
# Sourced after federation-e2e-common.sh, not run. The gateways need the
# `e2e-app` hosted login client and APP_REDIRECT (see federation-link-e2e.sh).

# ? ---------------------------------------------------------------------------
# ? Browser helpers
# ? ---------------------------------------------------------------------------

# hidden <name>: the value of a hidden input of the page on stdin
hidden() {
    python3 -c 'import re,sys; m=re.search(r"name=\"%s\" value=\"([^\"]*)\"" % sys.argv[1], sys.stdin.read()); print(m.group(1) if m else "")' "$1"
}

# location <headers file>: the Location of a redirect
location() { tr -d '\r' <"$1" | awk 'tolower($1) == "location:" { print $2 }'; }

# query_param <url> <name>
query_param() {
    python3 -c 'import sys,urllib.parse as u; print(u.parse_qs(u.urlparse(sys.argv[1]).query)[sys.argv[2]][0])' "$1" "$2"
}

# field <dotted path>: a field of the JSON on stdin (list indexes are
# numbers); prints None when absent
field() {
    python3 -c '
import json, sys
value = json.load(sys.stdin)
for key in sys.argv[1].split("."):
    if isinstance(value, list) and key.isdigit() and int(key) < len(value):
        value = value[int(key)]
    elif isinstance(value, dict):
        value = value.get(key)
    else:
        value = None
print(value)' "$1"
}

# emailed_code <database> <user> <domain>: the code of the newest session
emailed_code() {
    sql "$1" "select meta->'emailCode'->>'code' from token
              where meta->'emailCode'->'email'->>'username' = '$2'
                and meta->'emailCode'->'email'->>'domain' = '$3'
              order by expiration desc limit 1"
}

# hosted_login <base> <database> <email> <jar> <authorize url>: sign in with
# the emailed code; prints where the hosted login sends the browser back
hosted_login() {
    local base="$1" database="$2" email="$3" jar="$4" url="$5" page csrf
    page="$(curl -c "$jar" -b "$jar" "$url")"
    csrf="$(hidden csrf_token <<<"$page")"
    [ -n "$csrf" ] || fail "no hosted login page at $url"
    page="$(curl -c "$jar" -b "$jar" -H "Origin: $base" --data-urlencode "csrf_token=$csrf" \
        --data-urlencode "email=$email" "$base/_adm/hosted-login/email")"
    csrf="$(hidden csrf_token <<<"$page")"
    curl -c "$jar" -b "$jar" -H "Origin: $base" -D "$WORKDIR/code.headers" -o /dev/null \
        --data-urlencode "csrf_token=$csrf" \
        --data-urlencode "code=$(emailed_code "$database" "${email%@*}" "${email#*@}")" \
        "$base/_adm/hosted-login/code"
    location "$WORKDIR/code.headers"
}

# bearer <base> <database> <email>: a session token of the e2e app client
bearer() {
    local base="$1" verifier challenge back code
    verifier="$(openssl rand -hex 32)"
    challenge="$(printf '%s' "$verifier" | openssl dgst -sha256 -binary | basenc --base64url | tr -d '=')"
    back="$(hosted_login "$base" "$2" "$3" "$WORKDIR/jar-app-$RANDOM" \
        "$base/_adm/hosted-login/authorize?response_type=code&client_id=e2e-app&redirect_uri=$APP_REDIRECT&state=e2e&code_challenge=$challenge&code_challenge_method=S256")"
    code="$(query_param "$back" code)"
    curl --data-urlencode grant_type=authorization_code --data-urlencode "code=$code" \
        --data-urlencode "redirect_uri=$APP_REDIRECT" --data-urlencode client_id=e2e-app \
        --data-urlencode "code_verifier=$verifier" "$base/_adm/hosted-login/token" |
        json_field access_token
}

# rpc <base> <token> <method> <params json>: the JSON-RPC response
# (an empty token sends no Authorization header)
rpc() {
    local auth=()
    [ -z "$2" ] || auth=(-H "Authorization: Bearer $2")
    curl "${auth[@]}" -H 'Content-Type: application/json' \
        -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$3\",\"params\":$4}" "$1/_adm/rpc"
}

# account_token <base> <database> <email>: a token whose profile has an account
account_token() {
    local token
    token="$(bearer "$1" "$2" "$3")"
    rpc "$1" "$token" beginners.accounts.create '{"name":"e2e"}' >/dev/null
    printf '%s' "$token"
}
