# Feature Spec: Hosted Email Login (gateway-rendered magic link flow)

**Milestone:** M3 — Auth Evolution (follow-up to `magic-link-auth`)
**Scope:** Gateway (Rust) + templates; consumers (webapp, other fronts) as clients
**Status:** First slice implemented (uncommitted) — see design.md

---

## Problem

Today the gateway exposes the magic link flow only as endpoints
(`/magic-link/request`, `/magic-link/display`, `/magic-link/verify`) plus the
email and display templates. Every consumer system builds its own "type your
email" and "type the code" screens and calls `verify` itself to get the JWT.

Goal: an **optional** mode where the whole email identity-verification cycle
runs on gateway-hosted pages, and the consumer front only receives the
result at the end. The current endpoint-only mode stays.

---

## Feasibility verdict

**Feasible, and an established pattern.** This is a hosted login page
("universal login") handing the result back to the client through the
**OAuth 2.0 Authorization Code grant with PKCE** (RFC 6749 §4.1, RFC 7636).
The OAuth 2.0 Security BCP (RFC 9700) recommends exactly this for browser
apps, and the webapp's Auth0 login already works this way.

Done correctly it is **more secure** than the current model. The
security-sensitive UI (OTP entry, attempt limits, CSRF, anti-framing) lives
in one audited place instead of being reimplemented by every consumer. The
current model is not safer, it only spreads that responsibility around.

The risk is in the hand-back: a naive "redirect with the JWT in the URL" is
insecure. See the non-negotiable constraints below.

---

## Findings in the current flow (independent of this feature)

| # | Finding | Evidence | Severity |
|---|---|---|---|
| F1 | **No attempt limit on `verify`.** After the display link is opened, the 6-digit code (10^6 space) can be guessed for the whole TTL with no counter and no rate limit. | `verify_magic_link.rs`, `get_and_invalidate_magic_link_code` (lookup by `(email, code)`, no failure tracking); rate limiting deferred to M4 in `magic-link-auth/spec.md` | High |
| F2 | **TOTP bypass.** `verify_magic_link_url` always returns `totp_required: false`. The password `/login` path checks `user.mfa().totp`. A user with TOTP enabled can be logged in with email access alone. | `user_endpoints.rs` (`verify_magic_link_url` vs `check_email_password_validity_url`) | High (MFA downgrade) |
| F3 | **GET consumes the token.** `/magic-link/display` invalidates the token on GET. Corporate link scanners (Safe Links, etc.) prefetch email links: the token is burned, the user gets the error page, and the scanner received the code in the HTML. | `display_magic_link_url` → `get_code_and_invalidate_display_token` | Medium (availability; code exposure to scanner) |
| F4 | **Gateway HTML pages are frameable.** No `X-Frame-Options` / `Content-Security-Policy: frame-ancestors` anywhere in `ports/` or `lib/`. | grep, no matches | Medium once hosted forms exist (clickjacking) |
| F5 | `templates/web/` is not split by locale (`en-us/`, `pt-br/`, `es/` exist only for email). | `templates/` tree | Low, but blocks hosted pages |

F1 and F2 should be tracked as issues of their own whatever is decided here.
The hosted flow fixes F1 by design (OTP bound to the browser session plus an
attempt cap) and must resolve F2 explicitly.

---

## Target flow (Authorization Code + PKCE)

```mermaid
sequenceDiagram
    participant F as Consumer front
    participant B as Browser
    participant G as Gateway (hosted pages)
    participant M as Mailbox

    F->>F: generate code_verifier, state
    F->>B: redirect to G /authorize?client_id&redirect_uri&state&code_challenge(S256)
    B->>G: GET /authorize
    G->>G: validate client_id + exact redirect_uri; create login session (cookie)
    G->>B: hosted page: enter email
    B->>G: POST email (+ CSRF token)
    G->>M: email with OTP (and/or link)
    B->>G: POST OTP (+ CSRF token), bound to the session cookie, attempts capped
    G->>G: issue authorization code (single-use, ~60s, bound to client, redirect_uri, code_challenge)
    G->>B: 302 redirect_uri?code&state
    B->>F: code, state
    F->>F: check state
    F->>G: POST /token {code, code_verifier, redirect_uri, client_id}
    G->>F: MyceliumLoginResponse (JWT)
```

---

## Requirements (draft)

### HEL-001 — Client registry
The gateway knows which clients may use the hosted flow, each with an
**exact-match** allowlist of `redirect_uri`s. `allowedOrigins` (CORS) does
not replace this. No such registry exists today; it is the largest new
component. Storage location is an open decision (G1).

### HEL-002 — `GET /authorize`
Validates `client_id`, `redirect_uri` (exact match), `response_type=code`,
`code_challenge_method=S256`, and that `state` is present. If `client_id` or
`redirect_uri` is invalid, render an error page and **never redirect** (no
open redirect). Otherwise create a short-lived login session (HttpOnly,
Secure, SameSite=Lax cookie on the gateway domain) and render the email page.

### HEL-003 — Hosted email page → OTP dispatch
POST with a CSRF token. Reuses `request_magic_link` machinery. Always shows
"if the address is valid, a code was sent" (no enumeration, same as today).

### HEL-004 — Hosted OTP page
POST with CSRF token. The OTP is valid **only together with the login
session that requested it**. Attempt cap per session and per email (e.g. 5)
invalidates the OTP. Constant-time comparison.

### HEL-005 — Authorization code issuance
On success: create an authorization code (random ≥128 bits, single-use,
TTL ≈ 60 s) bound to `client_id`, `redirect_uri`, `code_challenge` and the
user, then `302` to `redirect_uri?code=…&state=…`.

### HEL-006 — `POST /token`
Exchanges `code` + `code_verifier` + `redirect_uri` + `client_id` for the
same `MyceliumLoginResponse` that `/magic-link/verify` returns today.
Verifies PKCE (S256), consumes the code atomically, and rejects reuse.

### HEL-007 — MFA
If the user has TOTP enabled, the hosted flow asks for it **before** issuing
the authorization code (fixes F2 for this path).

### HEL-008 — Hosted page hardening
`Content-Security-Policy` (including `frame-ancestors 'none'`),
`X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`,
`Cache-Control: no-store` on every hosted page.

### HEL-009 — i18n
Hosted pages under per-locale templates (`templates/<locale>/web/…`), with
the locale chosen from `ui_locales` / `Accept-Language`.

### HEL-010 — Coexistence
The current endpoints keep working unchanged. Hosted mode is opt-in per
client.

---

## Non-negotiable security constraints

1. **Never put the JWT in a redirect**, neither in the query nor in the
   fragment. That would be the implicit grant, which RFC 9700 deprecates.
   Only the one-time authorization code goes in the URL.
2. **PKCE S256 is mandatory** for every client. Plain is not accepted.
3. **Exact `redirect_uri` match** against the registry. No prefix or
   wildcard matching.
4. **`state` is required.** CSRF tokens on every hosted form.
5. Anti-framing and no-store headers on hosted pages (F4).
6. Attempt caps and rate limiting are a **prerequisite** for hosted pages,
   not an M4 deferral. Hosted pages are public by definition.
7. Link-based steps must not consume secrets on GET (F3). Consume on POST
   from an interstitial page.

---

## Gray areas (to decide before Design)

- **G1 — Where clients and redirect URIs live:** gateway config file
  (simple, static) / tenant metadata (per-tenant self-service) / a new
  `oauth_client` table (most flexible, needs admin endpoints and webapp UI).
- **G2 — Same-browser vs cross-device:** today the user can open the link on
  the phone and type the code on the desktop. An OTP bound to the login
  session keeps that (the code is typed into the session that started the
  flow), but a "click the link to log in" variant only works in the same
  browser. Decide whether the email carries just the OTP, just a link, or
  both.
- **G3 — Protocol surface:** minimal Mycelium-specific `/authorize` +
  `/token` (smaller; consumers use the SDKs) vs standards-compliant OAuth 2.1
  / OIDC (discovery document, `id_token`, JWKS). Compliance lets consumers
  use off-the-shelf client libraries, but costs much more. Note: the JWT is
  HS512 today, so OIDC with third-party verification would need asymmetric
  signing.
- **G4 — Branding:** a single gateway look vs per-client or per-tenant
  branding (logo, colors) on hosted pages.
- **G5 — Gateway-domain SSO session:** whether a successful hosted login
  leaves a gateway session cookie so later `/authorize` calls skip the email
  step (real SSO), or every login starts from zero.

---

## Out of scope

- Fixing F1/F2/F3 in the existing endpoints (separate issues).
- Social/external IdP buttons on the hosted page.
- Refresh tokens.
