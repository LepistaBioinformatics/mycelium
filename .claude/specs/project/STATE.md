# State

**Last Updated:** 2026-04-25
**Current Work:** HMAC Key Rotation **shipped** — PR #151 merged into `develop` (gateway HEAD `aae89c96`). Monorepo pointer `e955f50` on `main`.

---

## Recent Decisions (Last 60 days)

### AD-004: HMAC signing key decoupled from `token_secret`; KVR-versioned, no legacy fallback (2026-04-25)

**Decision:** Connection-string HMAC signing now uses a dedicated `hmac_secret` set keyed by
explicit version (`hmacPrimaryVersion` + `[[hmacSecrets]]`). The `KVR(u32)` bean is part of
the HMAC input (anti-downgrade). Tokens missing or carrying an unknown KVR are rejected
(`MYC00030` / `MYC00031`); there is **no implicit "missing KVR = v1"** fallback.

**Reason:** `token_secret` previously played two roles — KEK source (envelope encryption) and
HMAC signing key for connection strings. Rotating it atomically revoked every live connection
string, which is unacceptable when operators want to rotate the KEK or the signing key
independently. Versioning the HMAC key set makes rotation a routine operation: add a new
version, bump primary, wait for TTL drain, retire the old version.

**Trade-off:** **BREAKING.** Etapa 3 deploy permanently invalidates connection strings issued
under the old single-secret scheme — every active user must re-authenticate on next request.
The deployment warning is reproduced in three places: `settings/config.example.toml`, the
operator runbook (`docs/book/src/22-hmac-key-rotation.md`), and the changelog entry. Operator
must plan the rollout during a window where full re-auth is tolerable.

**Impact:** New CLI command `myc-cli rotate-kek` lets operators rotate the KEK without
touching connection strings (re-wraps every DEK in place; idempotent). Verification runs
**before** the DB lookup in `fetch_connection_string_from_request` middleware via
`Mac::verify_slice` (constant-time). New error codes: `MYC00030` (MissingKeyVersion),
`MYC00031` (UnknownKeyVersion), `MYC00032` (SignatureMismatch).

**PR:** #151 merged 2026-04-25 (4 commits: `d4e21e94`, `2a6ebfbf`, `b2d4685f`, `39151012`).
Spec at monorepo `.claude/specs/features/hmac-key-rotation/`.

---

### AD-001: Use `OnceLock<Result<Tera, String>>` instead of `lazy_static! + panic!` (2026-04-06)

**Decision:** Replace the `lazy_static!` Tera initialization (which called `panic!` on failure) with
`std::sync::OnceLock<Result<Tera, String>>`, initialized lazily and propagating errors to callers.

**Reason:** `OnceLock` is available since Rust 1.70 (already required by this crate), supports
fallible init, and avoids the `lazy_static` dependency pattern. `Tera::default()` + runtime error
was considered but rejected — it hides the init failure too silently.

**Trade-off:** Callers of the template accessor must now handle `Result`; slightly more boilerplate
at call sites.

**Impact:** All template-render call sites must propagate errors via `?` or explicit match.

---

### AD-003: Per-tenant secrets use AES-256-GCM encrypted at rest, not SecretResolver (2026-04-19)

**Decision:** Secrets that vary per tenant (Telegram bot token, webhook secret) are stored as
`base64(nonce ‖ AES-256-GCM ciphertext ‖ tag)` in the `tenant.meta` JSONB column. The encryption
key is derived from `AccountLifeCycle::token_secret` via SHA-256 (`derive_key_from_uuid`).
`SecretResolver<String>` is not used for this class of secrets.

**Reason:** `SecretResolver` requires the operator to format the stored value as JSON
(`"\"plain-token\""` for plain text, `{"env":"VAR"}` for env, `{"vault":{…}}` for Vault).
This is not documented in the field names and causes silent failures at runtime when the format
is wrong. Encrypted at rest gives a uniform, operator-friendly write path (plain string in,
ciphertext stored) with no format ambiguity.

**Trade-off:** If `AccountLifeCycle::token_secret` rotates, all per-tenant secrets encrypted
under the old key must be re-submitted via the config endpoint. No automatic re-encryption.

**Pattern to follow for future per-tenant secrets:**
- Write: call `encrypt_string(&plain, &config)` from `core::domain::utils` in the use case
- Read: call `decrypt_string(&ciphertext_b64, &config)` in the adapter constructor (eagerly)
- Store under a `TenantMetaKey` variant with a descriptive name (no `Ref` suffix)

---

### AD-002: Propagate `choose_host()` error at call sites (2026-04-06)

**Decision:** Changed `choose_host()` signature to return `Result<String, MappedErrors>` and updated
both call sites (`route.rs`, `load_operations_from_downstream_services.rs`) to use `?`.

**Reason:** The change to `service.rs` forced a signature change; updating call sites was mandatory,
not optional. Committed all 5 files together as one atomic change.

**Trade-off:** None — this was the only correct approach.

**Impact:** Any future call site adding `choose_host()` must handle the error.

---

## Active Blockers

_(none)_

---

## RPC ↔ REST Audit (2026-04-13)

Full audit of all 12 RPC dispatcher files (88 methods total) against their REST equivalents.
REST is the reference — it is validated; RPC is what may diverge.

### Fixed

**`beginners.accounts.create` (`BEGINNERS_ACCOUNTS_CREATE`)** — resolved in this session.

- REST `create_default_account_url` does not use `MyceliumProfileData` extractor; it calls
  `check_credentials_with_multi_identity_provider` directly from `req`.
- RPC `admin_jsonrpc_post` was extracting `profile: MyceliumProfileData` as an Actix extractor,
  which returned HTTP 403 before the handler body ran for users with a valid JWT but no account.
- Fix: profile extraction moved inside the handler body; `GatewayError::Forbidden` falls back to
  an anonymous profile (struct literal with `Uuid::nil()`), allowing the dispatcher to be reached.
  The dispatcher already re-validates credentials independently.
- File changed: `ports/api/src/rpc/handlers.rs` only.

### Remaining divergences

_(none — all resolved)_

**`service.listDiscoverableServices`** — resolved in this session.

- REST `GET /services/tools` is fully public (`security(())`; no `MyceliumProfileData`).
- RPC was blocking unauthenticated callers with `GatewayError::Unauthorized` before the dispatcher
  ran.
- Fix: added `GatewayError::Unauthorized(_)` alongside `GatewayError::Forbidden(_)` in the
  anonymous-profile fallback in `admin_jsonrpc_post`. Both now fall through to the dispatcher.
  Protected methods remain secure via internal dispatcher checks (`profile.acc_id`, `is_manager`,
  `is_staff`, etc.).

**Decision:** RPC must mirror REST visibility. If a REST endpoint is public, the equivalent RPC
method must also be reachable without authentication.

### Clean scopes (no divergences)

`managers`, `accountManager`, `guestManager`, `subscriptionsManager`, `systemManager`,
`tenantManager`, `tenantOwner`, `usersManager`, `staff`, `gatewayManager`, `service` — all 88
methods have consistent profile requirements, credential extraction patterns, and authorization
checks between RPC and REST.

---

## Lessons Learned

### L-002: Personal accounts vs subscription accounts — Telegram IdP model (2026-04-19)

**Context:** The original Telegram IdP spec (OQ-2b) stored identity on subscription accounts
(tenant-scoped, `account.tenant_id IS NOT NULL`). This was wrong: only personal accounts
(user/manager/staff, `account.tenant_id IS NULL`) can own cross-tenant identities.

**Problem:** `get_by_telegram_id` filtered `WHERE account.tenant_id = tenant_id`, which could
never find personal accounts. The per-tenant unique index `(telegram_user.id, tenant_id)` also
failed silently because `tenant_id` was NULL.

**Fix:** Global lookup (no `tenant_id` filter). Global unique index on `telegram_user.id` alone.
Login still scopes the issued connection string with the requested `tenant_id`.

**Rule:** Any identity or credential that must be valid across multiple tenants belongs on a
personal account, not a subscription account. Subscription accounts are inherently tenant-scoped.

---

### L-003: JWT Bearer vs connection string — different headers, never interchangeable (2026-04-20)

**Context:** Documentation for Telegram IdP used `Authorization: Bearer <connection_string>`, which
is wrong. Magic-link issues a JWT sent as `Authorization: Bearer <jwt>`. Telegram login issues a
connection string (`acc=...;tid=...;sig=...`) sent as `x-mycelium-connection-string: <string>`.

**Rule:** Never mix the two. A connection string sent as `Authorization: Bearer` fails JWT signature
validation and returns 401. The gateway checks `x-mycelium-connection-string` first, falls back to
Bearer only if absent — but the fallback is for JWT, not for connection strings.

**How to apply:** In documentation and client code, always use `Authorization: Bearer` for JWTs
(magic-link, email+password) and `x-mycelium-connection-string` for connection strings (Telegram
login, service tokens).

---

### L-001: Signature changes in domain DTOs ripple to call sites outside the feature scope (2026-04-06)

**Context:** The `fix-notifier-panics` spec listed 3 target files. Changing `choose_host()` to
return `Result` forced updates in `route.rs` and `load_operations_from_downstream_services.rs`,
which were not in the spec.

**Problem:** Spec scope was defined by panic sites, not by the full call graph of changed APIs.

**Solution:** Committed all 5 files together; spec traceability remained valid since the call-site
changes were mechanical (add `?`), not behavioral.

**Prevents:** Future specs that change a public DTO method should proactively grep call sites and
include them in scope.

---

## Quick Tasks Completed

| #   | Description                             | Date       | Commit       | Status  |
| --- | --------------------------------------- | ---------- | ------------ | ------- |
| 001 | fix-notifier-panics (medium)            | 2026-04-06 | `b41b381c`   | ✅ Done |
| 002 | RFC 7239 Forwarded header compliance    | 2026-04-18 | `6faa212f`   | ✅ Done |
| 003 | RUSTSEC-2026-0104 rustls-webpki pin     | 2026-04-25 | `39151012`   | ✅ Done |

---

## Current Focus

**HMAC Key Rotation — shipped** (2026-04-25). Spec `.claude/specs/features/hmac-key-rotation/`
at the monorepo root. PR #151 merged into `develop` (gateway HEAD `aae89c96`); monorepo
pointer `e955f50` on `main`.

| Etapa | Scope | Status | Commit |
|---|---|---|---|
| E1 | Decouple `hmac_secret` from `token_secret` (additive, fallback warn) | ✅ Done | `d4e21e94` |
| E2 | `myc-cli rotate-kek` (re-wrap DEKs, no token invalidation) | ✅ Done | `2a6ebfbf` |
| E3 | KVR versioned signing — **BREAKING** (no fallback) | ✅ Done | `b2d4685f` |
| Hotfix | RUSTSEC-2026-0104 pin (`rustls-webpki >= 0.103.13`) | ✅ Done | `39151012` |

**Open follow-ups:**
- **E2-T4** (Postgres integration test for `rotate-kek`) — deferred to roadmap M2 "Database
  Integration Tests" (no Postgres test scaffold in repo yet). Migrator covered by helper-layer
  unit tests.
- **Operator deployment of Etapa 3** — separate coordination required. Etapa 3 invalidates
  every connection string issued before deploy; plan the rollout during a re-auth-tolerant window.

---

**Telegram IdP — implementation complete, conceptual fix applied** — branch `feat/messaging-platform-idp/telegram`.

| Task | Status | Commit |
|---|---|---|
| T13 — TelegramUser DTO + AccountMeta key | ✅ Done | `12f80f53` |
| T14 — TenantMeta keys + TelegramConfig trait | ✅ Done | `12f80f53` |
| T15 — POST /auth/telegram/link | ✅ Done | `12f80f53` |
| T16 — DELETE /auth/telegram/link | ✅ Done | `12f80f53` |
| T17 — POST /auth/telegram/login/{tenant_id} | ✅ Done | `12f80f53` |
| T18 — POST /auth/telegram/webhook/{tenant_id} | ✅ Done | `12f80f53` |
| Encrypted config — POST /tenant-owner/telegram/config | ✅ Done | `12f80f53` |
| Fix: personal-account model (OQ-2b superseded) | ✅ Done | `ef8a707e` |
| T19 — Mode B routing (identity_source on Route) | ✅ Done | `735ddaf` |
| Post-T19 — BodyIdpResolver trait + TelegramIdpResolver | ✅ Done | `afa5b915` |
| Post-T19 — Screaming-architecture rule (`.claude/rules/`) | ✅ Done | `afa5b915` |
| Post-T19 — `IdentitySource` moved to `identity_source.rs` | ✅ Done | `afa5b915` |
| Post-T19 — `prepare_body_idp_context` pipeline module | ✅ Done | `afa5b915` |
| Post-T19 — `06-downstream-apis.md` docs (`allowedSources`, `identitySource`, CORS clarification) | ✅ Done | `c2dd1251` |
| Docs — `10-alternative-idps.md` (admin + user journeys, real-world examples) | ✅ Done | `3f373249` |
| Docs — tenant config scope clarification (what works without config) | ✅ Done | `912fbfd2` |
| Docs — JWT vs connection-string disambiguation | ✅ Done | `64c8d866` |

**Key decisions:**
- Secrets stored as AES-256-GCM ciphertext (`base64(nonce‖ct‖tag)`) — not plain text, not Vault ref
- Key derived from `AccountLifeCycle::token_secret` (same pattern as `HttpSecret`)
- `TelegramBotTokenRef` / `TelegramWebhookSecretRef` renamed to `TelegramBotToken` / `TelegramWebhookSecret`
- `TelegramConfigSvcRepo::from_tenant_meta` is now `async`, decrypts eagerly
- **OQ-2b superseded (2026-04-19):** Telegram identity links to personal accounts (user/manager/staff), not subscription accounts. Personal accounts have no `tenant_id` column. `get_by_telegram_id` is a global lookup. The unique DB index is now global (`idx_account_meta_telegram_user_id_global`). Login still scopes the connection string to the requested tenant.
- `AllowedAccounts(vec![])` bug fixed in `link_telegram_identity` and `unlink_telegram_identity` — was generating `WHERE id IN ()` (always false)

**M3 — Magic Link Auth ✅ Complete** — GT0–GT7 implemented. Spec updated to `Status: Implemented` (2026-04-18).

**M1 — Stability & Safety (in progress)**

| Item | Status |
|---|---|
| Panic elimination (notifier + boot) | ✅ Complete |
| RFC 7239 Forwarded header compliance | ✅ Complete (`6faa212f`) |
| JWT secret validation at startup | Planned |
| Router & auth middleware tests | Planned |
| mTLS client certificate auth | Planned |

### Implementation notes

- `MagicLinkTokenMeta` lives in `core/src/domain/dtos/token/token/magic_link_token.rs`
  (new submodule under `token/token/`), not in `meta/` (which only contains `UserRelatedMeta`)
- `verify_magic_link` use case returns `User` (not `(String, Duration)`) — JWT encoding
  happens in the REST handler, following the existing `check_email_password_validity` pattern
- The display token is invalidated with `jsonb_set(meta, '{token}', 'null'::jsonb)`
- `tera = "1"` added directly to `ports/api/Cargo.toml` (not in workspace deps)
- `verify_magic_link_url` uses `get_not_redacted_user_by_email` for fetching the user

## Deferred Ideas

- [x] Audit remaining `unwrap()` calls across the full codebase (test code excluded) — **Audited 2026-04-06**
  - Systemic issue: Diesel ORM layer uses `unwrap()` pervasively for type conversions from DB records
  - ~215 JSON serde (`from_value`/`to_value` on JSONB columns) — medium risk, panics on corrupt DB data
  - ~174 timestamp (`and_local_timezone(Local).unwrap()`) — low risk, only on DST ambiguity
  - ~47 DB string parse (`Uuid::from_str`, `Email::from_string`, `.parse()` on DB values) — medium risk
  - ~30 `Mutex::lock().unwrap()` — low risk, only on lock poisoning
  - ~99 static literal `from_str("...")` — zero risk, compile-time safe
  - ~8 SSL/startup fail-fast in `main.rs` — acceptable
  - Recommended: create a dedicated M1/M2 feature to harden the Diesel adapter layer with proper `?`-propagation
- [ ] `TelegramConfig` trait está em `core/domain/entities` mas nenhum use case do core a usa — apenas o port handler a consome diretamente via shaku. Isso viola o espírito da arquitetura hexagonal (traits no core deveriam ser portas para use cases, não para ports). Opções: mover o trait para `adapters/service` como tipo concreto, ou criar um use case de "resolve config" que o port chame. Capturado durante: Telegram IdP (2026-04-19)
- [ ] Email address validation in the DTO layer (not just at send time) — Captured during: fix-notifier-panics
- [ ] Hot-reloading Tera templates (ops/config concern) — Captured during: fix-notifier-panics

---

## Todos

_(none)_

---

## Preferences

**Model Guidance Shown:** never
