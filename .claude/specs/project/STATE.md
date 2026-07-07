# State

**Last Updated:** 2026-07-06
**Current Work:** M3 — Standalone Mode (specified; branch `feat/standalone-mode`). M1 ongoing.

---

## Recent Decisions (Last 60 days)

### AD-005: Standalone mode is a compile-time feature, not a runtime `mode` flag (2026-07-06)

**Decision:** Standalone (SQLite + moka + stub/file email + autogen secrets) is selected by a
`standalone` cargo feature, mutually exclusive with the default `postgres-backend`, producing a
separate binary/image. The roadmap's original `mode = "standalone"` runtime flag is rejected.

**Reason:** Every table's schema uses Postgres-only Diesel SQL types (`Uuid`, `Timestamptz`, `Jsonb`,
`Array<Text>`, `Array<Jsonb>`). These do not compile against Diesel's SQLite backend. A single
runtime-switchable binary would require storing them as TEXT in Postgres too, changing full mode
(violates "don't break full"). `MultiConnection` (Diesel 2.3.7) cannot span `Array`/`Jsonb` either.

**Trade-off:** Two binaries to build/ship; a parallel SQLite schema + model set + SQLite variants of
~44 repository impls (the dominant cost — close to a second repository-layer implementation).

**Corrections captured in spec:** (a) no Redis pub/sub exists — the only queue is an LPUSH email
list; (b) no rate-limiting / AI-retry-loop detection exists (brief limitation #1 is moot); (c) token
invalidation is DB-backed, not Redis (limitation #3 overstated); (d) `mem_db` does not implement the
KV traits, so a new `moka` adapter is required; (e) today there are 3 hard boot deps (PG+Redis+SMTP),
not "single dependency (PostgreSQL)".

**Secrets:** auto-generate once and persist (keyring-preferred, encrypted-file fallback). Regenerating
`token_secret` per restart would rotate the KEK and invalidate all connection strings (see AD-004).
Open concern: keyring is usually unavailable on the container/air-gapped/edge targets, so the file
fallback is the de-facto primary path.

**Spec:** `.claude/specs/features/standalone-mode/` (spec.md + design.md + tasks.md — 27 tasks in 8
groups; critical path is G2 SQLite backend). No implementation yet. All work is on the dedicated
branch **`feat/standalone-mode`** (gateway submodule) — never `develop` (user instruction 2026-07-06).


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

**Standalone Mode — specified, ready for execution** (2026-07-06). Branch **`feat/standalone-mode`**
(gateway submodule) — NEVER work on `develop` for this feature (user instruction). Spec at
`.claude/specs/features/standalone-mode/` (spec.md + design.md + tasks.md). See AD-005 for rationale.
**Tracking epic: GitHub issue [#159](https://github.com/LepistaBioinformatics/mycelium/issues/159)**
— reformatted 2026-07-07 per user request to follow the org's feature-request draft pattern (see
project #5 item "Harden the release pipeline..." as the reference example): title `[FEAT] ...`,
sections "Is your feature request related to a problem?" / "Describe the solution you'd like" /
"Describe alternatives you've considered" / "Additional context", plus an **Acceptance checklist**
using requirement IDs (`SM-R1..R15`) phrased as WHEN/THEN clauses (only check a criterion when it is
genuinely, fully satisfied — not partially). The granular 27-task implementation checklist
(`SM-T1..T27`) is kept as a separate section below the acceptance checklist for day-to-day tracking.
Local copy of the current issue body: `/tmp/standalone-issue.md` (regenerate via `gh issue view 159
--json body -q .body` if lost). Keep both the task checkboxes AND the acceptance checklist in sync
as work lands — task checkbox ≠ requirement satisfied; check `SM-Rn` only when its WHEN/THEN holds
end-to-end.

| Phase | Status |
|---|---|
| Specify (spec.md — requirements + brief corrections C-1…C-7) | ✅ Done |
| Design (design.md — compile-time feature, SQLite/moka/email/secrets/docker) | ✅ Done |
| Tasks (tasks.md — 27 tasks, 8 groups; critical path G2 SQLite) | ✅ Done |
| Execute — SM-T1 (feature scheme + guards) | ✅ Done (committed `ce493b53`) |
| Execute — SM-T2 (CI builds both targets) | ✅ Done (committed `4f7f9354`) |
| Execute — SM-T3 (sqlite feature on `myc-diesel`; backend isolation verified) | ✅ Done (committed `4d36fc94`) |
| Execute — SM-T4/T5/T6 (sqlite scaffolding: pool+pragmas, schema+migrations, type helpers) | ✅ Done, verified |
| Execute — SM-T7 (account + account_tag SQLite repos, block 1/3) | ✅ Done (committed `5721c643`) |
| Execute — SM-T8 (tenant + tenant_tag SQLite repos, block 2/3) | ✅ Done (committed `34300a06`) |
| Execute — SM-T9 (user, block 3/3 — closes block 1) | ✅ Done (committed `2f7ad14a`) |
| Execute — SM-T10 (token; session_token confirmed dead code, skipped) | ✅ Done (committed `7a050e83`) |
| Execute — SM-T11 (guest_role + guest_user + guest_user_on_account) | ✅ Done (committed `7504af86`) |
| Execute — SM-T12 (message) | ✅ Done (committed `a8c47288`) |
| Execute — SM-T13 (webhook + webhook_execution) | ✅ Done (committed `4093da0e`) |
| Execute — SM-T14 (error_code) | ✅ Done (committed `978df130`) |
| Execute — SM-T15 (licensed_resource + ProfileFetching) | ✅ Done (committed `46904cfc`) |
| Execute — SM-T16 (encryption_key — closes G2 per-entity repos) | ✅ Done, verified |
| Architectural correction — extract SQLite adapter into `adapters/diesel_sqlite` (own crate, not nested in `mycelium-diesel`) | ✅ Done, verified |
| Execute — SM-T17 (SqlAppModule sqlite wiring — barrier) | ⏳ Next |

**SM-T1 result:** `ports/api` now has `default=["postgres-backend"]` + no-op `standalone` marker + two
`compile_error!` guards. `cargo check` verified for default, `--no-default-features --features standalone`,
and the both-features failure. `fmt --check` clean. Not committed yet (awaiting user test/approval).

**SM-T7 result:** `adapters/diesel_sqlite/src/{models,repositories}/` account + account_tag trees
(details + landmines in tasks.md SM-T7 result block; path shown post architectural-correction — see
below). Key reusable lessons for T8+: enable `diesel/returning_clauses_for_sqlite_3_35`;
`account_type::jsonb @>` needs case-by-case full-equality vs. `json_extract` partial-match analysis,
don't assume uniform rewrite; `NaiveDateTime` Display/FromStr aren't symmetric (use the new
`naive_timestamp_{to,from}_text` pair); tables with `DEFAULT gen_random_uuid()` on postgres need the
app to supply the id explicitly on sqlite inserts. Integration test pattern established:
`test_support.rs` (temp DB + migrations) + per-entity lifecycle test. Not committed yet (awaiting user
test/approval).

**Architectural correction (post SM-T16):** the user flagged that SM-T3..T16 were built as a
`#[cfg(feature="sqlite")]`-gated `sqlite/` submodule nested inside `mycelium-diesel` — a structural
error, since every other adapter (`kv_db`, `mem_db`, `notifier`, `service`, `shared`) is its own
sibling crate under `adapters/`. Extracted the SQLite adapter into a new crate **`adapters/diesel_sqlite`**
(package `mycelium-diesel-sqlite`, lib `myc_diesel_sqlite`): moved all `sqlite/*` source + migrations,
flattened `crate::sqlite::` → `crate::` across 68 files, reverted `mycelium-diesel`
(`adapters/diesel`, later renamed to `adapters/diesel_postgres` to mirror `diesel_sqlite`'s naming —
package/lib names unchanged: `mycelium-diesel`/`myc_diesel`) to a plain unconditional Postgres-only
crate (no features, no cfg-gating), added
`mycelium-diesel-sqlite = { path = "adapters/diesel_sqlite" }` to the workspace
`[workspace.dependencies]` (picked up automatically by the `adapters/*` members glob). Verified:
`cargo build`/`test -p mycelium-diesel-sqlite` (22 tests green), `cargo build -p mycelium-diesel`
clean, `cargo build --workspace` clean, `cargo test --workspace --all` (0 failed across all crates),
`cargo fmt --all -- --check` clean (after `cargo fmt --all` reflowed the shortened `crate::` paths),
and `cargo build -p mycelium-api --no-default-features --features standalone` unaffected. **Standing
rule for all future adapter work in this codebase: new backend adapters are always separate sibling
crates under `adapters/`, never a feature-gated submodule nested inside an existing adapter crate**
(applies to SM-T18's future cache crate too). Not committed yet (awaiting user test/approval).

**SM-T8 result:** tenant + tenant_tag SQLite repos, reusing `account/shared.rs::created_at_from_text`
and `json_array_to_text`/`from_text`. Found and fixed a missing `joinable!(owner_on_tenant -> user
(owner_id))` in sqlite/schema.rs (present in postgres, missed in SM-T5). Discovered (not fixed, see
tasks.md SM-T8 result) a dormant postgres write-path bug in tenant status serialization, and two
JSON filters in `filter_tenants_as_manager` needing SQLite-specific (non-byte-identical but
practically-equivalent) rewrites. User directive: "pode seguir até terminar a implementação da
feature" — continue through all remaining tasks (SM-T9..T27) without stopping for per-task
confirmation. Following the pattern already established and accepted earlier in this session
(commit after each task once fully gate-checked: fmt + full-mode build/test + sqlite tests), continue
committing at each verified step rather than batching everything into one giant uncommitted diff —
easier to review/bisect. Never skip the build/test gate regardless.

**Block 1 (account + tenant + user) is done** — 17 repo impls, ~2,900 lines, 3 end-to-end lifecycle
tests (see tasks.md SM-T9 result for the full summary and reusable patterns established).

**SM-T10 result:** token repos done (registration/fetching/invalidation/deletion). Confirmed via grep
that `session_token` (core traits `SessionTokenRegistration`/`Fetching`/`Deletion`) has **zero
implementations anywhere in the codebase** — not postgres, not wired into any shaku module, not
consumed by any use-case. Vestigial dead trait code; removed from the standalone scope, nothing to
port. First genuine JSON1 *mutation*: `jsonb_set(meta,'{token}','null'::jsonb)` → `json_set(meta,
'$.token', json('null'))` (magic-link phase-1 consumption) — verified end-to-end via the new
two-phase-consumption test. Also closed a latent SQL-injection gap: postgres's token_invalidation/
token_deletion build raw SQL via inconsistent string interpolation (2 of 4 methods escape quotes, 2
don't); the SQLite port binds all parameters uniformly instead.

**SM-T11 result:** guest_role + guest_user + guest_user_on_account done. Key lesson (see tasks.md
SM-T11 result for full detail): **timestamp convention is per-table, not per-feature** — `guest_user`'s
own table uses genuine `DateTime<Local>` round-trip (needs `timestamp_to_text`/`from_text`), while
`guest_role`/`guest_role_children`/`guest_user_on_account` use the naive-reinterpretation convention
(needs `naive_timestamp_to_text`/`from_text`) — always check the postgres model field type
(`NaiveDateTime` vs `DateTime<Local>`) before picking a helper, never assume uniformity across a
whole feature. Also: Diesel's `.nulls_last()` is postgres-only — use `ORDER BY (col IS NULL), col
DESC` instead. Confirmed `guest_role_children`/`guest_user_on_account` are two more tables relying on
postgres server-side defaults (`created`) with no SQLite equivalent — explicit `created` on every
insert into these tables (growing list since SM-T7's `manager_account_on_tenant` finding: **always
check for server-side defaults before writing a SQLite insert**).

**G2 per-entity repos are all done** (SM-T7..T16 — 14 entity groups, ~44 postgres repo impls now
have SQLite equivalents; `session_token` excluded as dead code).

**Next action:** SM-T17 — `SqlAppModule` (sqlite) shaku wiring, the barrier task assembling every
`repositories::*` component built so far into one shaku module (mirroring postgres's `SqlAppModule`
in `adapters/diesel_postgres/src/repositories/mod.rs`), now authored in
`adapters/diesel_sqlite/src/repositories/mod.rs` — plain module, no cfg-gating (own crate). This
closes G2. Then G3 (SM-T18/19 — moka cache, **its own sibling crate under `adapters/`**), G4
(SM-T20/21 — local email transport), G5 (SM-T22 — autogen secrets), G6 (SM-T23/24 — standalone
config + cfg-gated `initialize_modules`, finally wiring `mycelium-diesel-sqlite` into `ports/api`),
G7 (SM-T25/26 — Dockerfile + E2E smoke), G8 (SM-T27 — docs), per user's "continue to completion"
directive. Build gate every step: full `cargo build --workspace` + `cargo test --workspace --all` +
`cargo fmt --all -- --check` + standalone binary build.

**Needs / reminders to resume:**
- Work only on `feat/standalone-mode`; keep full mode byte-identical after every task (SM-R14).
- G2 (SQLite backend) is the dominant cost — ~44 repo impls, two portability axes (types + JSON1
  pg-isms per OC-4). Land per-entity groups incrementally, full mode green throughout.
- Secrets must be generated once and persisted (regenerating rotates the KEK — see AD-004); keyring
  usually absent on targets → encrypted-file fallback is primary (OC-2).
- Commit-validation rule stands: no code commit until the user tests and approves.

---

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

## Release Automation (2026-04-26)

### Completed

| Item | Status | Detail |
|---|---|---|
| `release-prerelease.yml` | ✅ Done | `workflow_dispatch` on `develop` — bumps `beta` / `rc` via `cargo release` |
| `release-stable.yml` | ✅ Done | `workflow_dispatch` on `main` — bumps `patch` / `minor` / `major` via `cargo release` |
| `docker-release.yml` | ✅ Done | Triggers on tag push + `workflow_dispatch`; builds from `Dockerfile.dev`; pushes to `ghcr.io/LepistaBioinformatics/mycelium` |
| First pre-release image | ✅ Done | `8.3.1-rc.2` built and pushed to GHCR manually (tag push webhook missed) |

### crates.io publish — ✅ Complete (2026-04-26)

All 13 workspace crates confirmed to exist on crates.io (no name conflicts). Workflows updated:
- `release-prerelease.yml` and `release-stable.yml`: removed `--no-publish` from execute step, added `CARGO_REGISTRY_TOKEN` env at job level
- `docker-release.yml`: switched `file: Dockerfile.dev` → `file: Dockerfile`, added `build-args: VERSION=${{ steps.tag.outputs.version }}`

**Remaining manual step:** Add `CARGO_REGISTRY_TOKEN` secret to the GitHub repository settings before triggering the next release.


### Branch semantics

| Branch | Allowed release types | Notes |
|---|---|---|
| `develop` | `beta`, `rc` | Pre-release ladder |
| `main` | `patch`, `minor`, `major` | Stable only; `rc → stable` graduation runs here after PR merge |

### Image tagging strategy (GHCR)

| Tag format | Images produced |
|---|---|
| `8.3.2` (stable) | `:8.3.2`, `:latest` |
| `8.3.2-rc.1` | `:8.3.2-rc.1`, `:rc` |
| `8.3.2-beta.1` | `:8.3.2-beta.1`, `:beta` |

---

## Todos

- **Release pipeline hardening** — spec at `features/release-pipeline-hardening/spec.md` (2026-07-06). Root cause of GHCR↔GitHub-Release desync: no workflow creates GitHub Releases (only tags). Spec also covers crates.io Trusted Publishing (OIDC, drop long-lived `CARGO_REGISTRY_TOKEN`), image provenance + cosign signing, `workflow_dispatch` ref bug, and `v`-prefix naming drift. Next: Design → Tasks.
- Retroactively create GitHub Releases for `8.3.0`+ tags and publish/discard the stale `8.3.1-rc.2` Draft.

---

## Preferences

**Model Guidance Shown:** never
