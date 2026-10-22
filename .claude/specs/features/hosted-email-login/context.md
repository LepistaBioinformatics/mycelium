# Context: Hosted Email Login — decisions

**Date:** 2026-10-14

## Decided

### D1 — Brute force on the magic link code (F1): fix now
Implemented as a **failed-attempt cap stored with the code**, not a per-IP
HTTP limiter. Production runs multiple pods, so an in-memory limiter would be
per-pod and easy to spread across. The counter lives in the token record and
works in every build mode (Postgres, Postgres-only, standalone SQLite).

- `MagicLinkTokenMeta.failed_attempts` (`#[serde(default)]`, so records
  created before the change still deserialize).
- `MAX_FAILED_MAGIC_LINK_ATTEMPTS = 5`. A wrong code counts on every active
  record of the email, so opening more links does not grant more guesses.
- Lookup runs under a write lock (`FOR UPDATE` on Postgres,
  `immediate_transaction` on SQLite), so parallel guesses cannot race the
  counter.
- Code compared in constant time (`subtle`).
- Covers `/instance/bootstrap/complete` too (it reuses `verify_magic_link`).
- Endpoint contracts unchanged.

Not done: per-email throttle on `/magic-link/request` (email bombing).

### D2 — TOTP (F2): password login will be removed
Password login goes away as part of this feature, so magic link no longer
needs a TOTP step. Impact to plan for (not executed yet):

- Endpoints/RPC: `/login`, `start-password-reset`, `reset-password`,
  `beginners.users.checkEmailPasswordValidity`,
  `startPasswordRedefinition`, `checkTokenAndResetPassword`.
- TOTP endpoints and RPC methods (`totp/*`) become dead. **To confirm:**
  is removing TOTP implied?
- Seed staff CLI prompts for a password (`ports/cli/src/cmds/accounts.rs`,
  `create_seed_staff_account`). After removal, first login needs working
  email delivery.
- Existing `Provider::Internal(PasswordHash)` data: keep or migrate?
- Webapp reads `mfa.totp` (`Owners.tsx`, openapi schema).
- Breaking change for any consumer outside the monorepo calling `/login`.
- F2 remains live in released versions until the removal ships.

### D4 — Framing (F4): deny everywhere the gateway renders HTML
`X-Frame-Options: DENY` + `Content-Security-Policy: frame-ancestors 'none'`
on the four gateway-rendered pages (magic link display/error, bootstrap
claim/error) via `ports/api/src/rest/hosted_page_response.rs`. CSP limited
to framing on purpose: templates use inline styles and external fonts.
Proxied downstream responses are not touched.

## Still open

- F3 — `/magic-link/display` consumes the token on GET (link scanners).
- G1–G5 in `spec.md`.
- Disclosure route for F1 (private advisory vs public PR) before pushing.

## Hosted login slice decisions (2026-10-14, "siga")

G1 config file · G2 code-only email typed on the gateway page · G3 minimal
OAuth-compatible `/authorize` + `/token` with PKCE S256 · G4 single look ·
G5 no SSO. Password removal (D2) is a later, separate step. TOTP question
still open.

## Working tree ownership (nothing committed yet)

Keep these as separate commits/PRs:

- **F1 (attempt cap — possible private advisory):**
  `core/src/domain/dtos/token/token/magic_link_token.rs`,
  `core/src/domain/entities/token/token_invalidation.rs`,
  `adapters/diesel_postgres/src/repositories/token/token_invalidation.rs`,
  `adapters/diesel_sqlite/src/repositories/token/token_invalidation.rs`,
  `adapters/diesel_sqlite/src/repositories/token/token_registration.rs`
  (tests).
- **F4 (anti-framing):** `ports/api/src/rest/hosted_page_response.rs`,
  `ports/api/src/rest/instance/bootstrap_claim_page.rs`,
  `ports/api/src/rest/role_scoped/beginners/user_endpoints.rs` (the two
  page renders), `ports/api/src/rest/mod.rs` (the
  `hosted_page_response` line).
- **Hosted login:** everything else listed in `design.md`, plus
  `ports/api/src/rest/mod.rs` (the `hosted_login` line), `main.rs`,
  `verify_magic_link.rs` (extraction of `ensure_principal_user_by_email`).
