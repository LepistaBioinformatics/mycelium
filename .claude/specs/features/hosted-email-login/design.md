# Design: Hosted Email Login — first slice

Decisions taken (2026-10-14, see `context.md`): G1 = clients in the config
file, G2 = email carries only the 6-digit code typed on the gateway page,
G3 = minimal OAuth-compatible `/authorize` + `/token` with PKCE S256,
G4 = single look, G5 = no SSO session (every login starts from zero).

Deferred from this slice: i18n of the web pages (HEL-009; email template is
localized, pages are English), per-client branding, OIDC discovery, webapp/
SDK client helpers, removal of password login (D2).

---

## Flow

```
GET  /_adm/hosted-login/authorize   ?response_type=code&client_id
                                    &redirect_uri&state&code_challenge
                                    &code_challenge_method=S256
     → validate client + exact redirect_uri, response_type, S256, state
     → create session record, Set-Cookie myc_hosted_login=<sessionId>
     → render web/hosted-login-email.html (hidden csrf)

POST /_adm/hosted-login/email       form: csrf, email      (cookie)
     → session by cookie, csrf check
     → atomic (locked per email): session has no code yet AND fewer than 5
       active sessions hold a code for this email → attach
       {email, code, failedAttempts: 0}; otherwise refuse
       (one code per session: resending means a new /authorize)
     → send email/hosted-login-code
     → render web/hosted-login-code.html

POST /_adm/hosted-login/code        form: csrf, code       (cookie)
     → atomic: code check under lock; wrong → failedAttempts++ (5 → session
       refuses every code, kept until it expires); right → issue authorizationCode, expiration = now + 60 s
       and clear the email code
     → 303 redirect_uri?code=<authorizationCode>&state=<state>
       (wrong code → re-render code page with error; exhausted → error page)

POST /_adm/hosted-login/token       application/x-www-form-urlencoded
                                    (RFC 6749 §4.1.3): grant_type=
                                    authorization_code, code, redirect_uri,
                                    client_id, code_verifier
     → atomic consume by authorizationCode (single use, not expired)
     → client_id + redirect_uri equal to the session's, PKCE S256 matches
       (a failed check still burns the code)
     → ensure principal user for the email (same as magic link verify)
     → 200 { access_token, token_type: "Bearer", expires_in, user }
       errors: 400 { error: invalid_request | invalid_grant |
       unsupported_grant_type }
```

Errors before a redirect_uri is validated never redirect (no open redirect).

## Config

```toml
[auth.hostedLogin.define]
clients = [
  { clientId = "webapp", redirectUris = ["http://localhost:5173/callback"] },
]
```

`AuthConfig.hosted_login: OptionalConfig<HostedLoginConfig>`; disabled or
absent → no client is accepted. Front origins still need `allowedOrigins`
for the `/token` call (CORS).

## Core

`core/src/domain/dtos/hosted_login/`
| File | Type | Role |
|---|---|---|
| `hosted_login_client.rs` | `HostedLoginClient`, `HostedLoginClients` | registry; `allows(client_id, redirect_uri)` exact match |
| `hosted_login_request.rs` | `HostedLoginRequest` | client_id, redirect_uri, state, code_challenge; `pkce_matches(verifier)` (S256, constant time); `validate()` |
| `hosted_login_email_code.rs` | `HostedLoginEmailCode` | email, code, failed_attempts; `code_matches`, `register_failed_attempt` (cap 5) |
| `hosted_login_session.rs` | `HostedLoginSession` | session_id, csrf_token, request, email_code, authorization_code; `new(request)`, `csrf_matches` |
| `hosted_login_code_check.rs` | `HostedLoginCodeCheck` | `Verified(HostedLoginSession)` / `Rejected` / `Expired` |
| `hosted_login_code_attachment.rs` | `HostedLoginCodeAttachment` | `Attached` / `AlreadySent` / `EmailCapReached` / `NotFound` |

Secrets (`sessionId`, `csrfToken`, `authorizationCode`): 32 random bytes,
base64url without padding.

`core/src/models/hosted_login_config.rs` — `HostedLoginConfig { clients }`.

`core/src/domain/entities/hosted_login/` — ports:
- `HostedLoginSessionRegistration::create(session, expires)`
- `HostedLoginSessionFetching::get_by_session_id`
- `HostedLoginSessionUpdating::attach_email_code(session_id, email_code,
  max_active_per_email) -> HostedLoginCodeAttachment` (Attached /
  AlreadySent / EmailCapReached / NotFound), locked per email
  (`pg_advisory_xact_lock(hashtext(email))` / `immediate_transaction`)
- `HostedLoginSessionInvalidation::check_code_and_issue_authorization_code(
  session_id, code, authorization_code, expires)` (write-locked) and
  `consume_authorization_code(code)` (atomic fetch + delete)

`core/src/use_cases/role_scoped/beginner/hosted_login/`
- `start_hosted_login`
- `send_hosted_login_code` (csrf, atomic attach with cap, email)
- `verify_hosted_login_code` (csrf, atomic check)
- `exchange_hosted_login_code` (consume, client/redirect/PKCE, user)

`core/src/use_cases/role_scoped/beginner/user/ensure_principal_user_by_email.rs`
— extracted from `verify_magic_link` (fetch or create, upgrade to principal);
both flows call it.

## Adapters

Rows live in the `token` table (no migration). Meta = `HostedLoginSession`
JSON, identified by `meta->>'sessionId'` / `json_extract(meta,'$.sessionId')`.
Locking: `FOR UPDATE` (Postgres), `immediate_transaction` (SQLite).
One file per trait under `repositories/hosted_login/` in both
`diesel_postgres` and `diesel_sqlite`; registered in each `SqlAppModule`.

## Ports

`ports/api/src/rest/hosted_login/` — `mod.rs` (configure), one file per
endpoint, `hosted_login_session_cookie.rs` (HttpOnly, SameSite=Lax,
Path=/_adm/hosted-login, Secure when `domain_url` is https, Max-Age = TTL).
Pages via `hosted_page_response` (anti-framing) plus
`Cache-Control: no-store` and `Referrer-Policy: same-origin` (`no-referrer` makes browsers send `Origin: null` on the form POSTs, which CORS rejects).
Mounted in `main.rs` next to `instance`.

## Templates

`templates/web/hosted-login-email.html`, `hosted-login-code.html`,
`hosted-login-error.html` — plain HTML forms (no JS), same look as the
bootstrap pages. `templates/{en-us,pt-br,es}/email/hosted-login-code.jinja`.

## Security checklist

- JWT never in a URL; only the single-use 60 s authorization code.
- PKCE S256 only; exact redirect_uri match at authorize and at token.
- Code bound to the browser session (cookie); 5 attempts per session;
  max 5 unverified codes per email within the session TTL — exhausted
  sessions are kept until they expire so they keep counting. Bounds
  guesses to 25 and emails to 5 per address per TTL window (an attacker
  can use it to delay a victim's login — accepted).
- CSRF token on both forms + SameSite=Lax cookie.
- Anti-framing, no-store, same-origin referrer policy on every page.
- Constant-time comparisons for csrf, code and PKCE.

## Known concerns (not in this slice)

- Expired rows in `token` are never purged (no job deletes them). Every
  unauthenticated `GET /authorize` writes a row, so the table grows until a
  purge exists.
