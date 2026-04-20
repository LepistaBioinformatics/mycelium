# Telegram IdP — Task Breakdown

**Spec:** `telegram.md`  
**Date:** 2026-04-19  
**Status:** Ready for implementation

Tasks are ordered by dependency. A task may only start when all tasks listed in its "Depends on" field are complete. Tasks within the same group with no inter-dependencies can be parallelized.

---

## Group 0 — Investigation (no dependencies)

### T0 — Audit existing Cargo workspace dependencies

**File:** `Cargo.toml` (workspace root)  
**Action:** Check whether `hmac`, `sha2`, `secrecy`, `subtle` (or `constant_time_eq`) are already declared in `[workspace.dependencies]`.  
**Output:** A written note (comment in T1 or inline decision) stating which crates are new and which already exist.  
**Why first:** Adding duplicate deps with different versions breaks the build. Must check before T1.

**Acceptance:**
- [ ] List of crates to add (if any) confirmed
- [ ] No version conflicts found

---

## Group 1 — Foundations (parallel after T0)

### T1 — Add missing crate dependencies to workspace

**Depends on:** T0  
**Files:** `Cargo.toml` (workspace), `lib/http_tools/Cargo.toml`  
**Action:** Add only the crates confirmed missing in T0:
- `secrecy` — `SecretString` for bot token / webhook secret
- `subtle` — constant-time comparison (prefer over `constant_time_eq`)
- `hmac` + `sha2` — if not already present

Add a comment next to each new dep referencing the RUSTSEC advisory if applicable.

**Acceptance:**
- [ ] `cargo build -p myc-http-tools` passes after changes
- [ ] No duplicate version warnings in `cargo tree`

---

### T2 — SQL migrations

**Depends on:** nothing (pure SQL)  
**Files:** `adapters/diesel/sql/` (3 new files)

**T2a — GIN index on `account.meta`**
```sql
-- file: NNNN_account_meta_gin_index.sql
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_account_meta_gin
ON account USING GIN (meta jsonb_path_ops);
```

**T2b — Unique index `(telegram_user.id, tenant_id)`**
```sql
-- file: NNNN_account_telegram_unique_per_tenant.sql
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS
    idx_account_meta_telegram_user_id_per_tenant
ON account ((meta -> 'telegram_user' ->> 'id'), tenant_id)
WHERE meta ? 'telegram_user';
```

**T2c — Audit table**
```sql
-- file: NNNN_telegram_identity_audit.sql
CREATE TABLE telegram_identity_audit (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL,
    account_id  UUID NOT NULL,
    event       TEXT NOT NULL CHECK (event IN ('linked','unlinked','login_ok','login_fail')),
    telegram_id BIGINT,
    ip          INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON telegram_identity_audit (tenant_id, account_id);
CREATE INDEX ON telegram_identity_audit (created_at);
```

**Acceptance:**
- [ ] All three migrations run cleanly against a local dev DB
- [ ] `\d account` shows both new indexes
- [ ] `\d telegram_identity_audit` shows correct columns and check constraint
- [ ] `CONCURRENTLY` present on both indexes (no table lock)

---

### T3 — Extend `TenantMetaKey` with Telegram variants

**Depends on:** nothing  
**File:** `core/src/domain/dtos/tenant/meta.rs`  
**Action:** Add two variants:
```rust
TelegramBotTokenRef,      // Vault path to bot token
TelegramWebhookSecretRef, // Vault path to webhook secret
```
Add `Display`, `FromStr`, and `Serialize` arms for both. Follow the exact pattern of existing variants.

**Acceptance:**
- [ ] `cargo test -p myc-core` passes
- [ ] Round-trip serialization test for both new variants
- [ ] Values stored are Vault path strings, not secret values (documented in variant doc comment)

---

### T4 — Verify `AccountMetaKey::TelegramUser` stored shape

**Depends on:** nothing  
**File:** `core/src/domain/dtos/account/meta.rs`  
**Action:** `TelegramUser` variant already exists. Confirm (or add) a doc comment specifying the expected JSON shape stored in `account.meta`:
```json
{ "id": 123456789, "username": "john_doe" }
```
`username` is nullable (`null` or absent). `id` is `i64`, never null.  
No code changes needed if the variant exists and is correct — this task is a verification gate.

**Acceptance:**
- [ ] Doc comment on `TelegramUser` variant states the stored JSON shape
- [ ] `cargo test -p myc-core` passes

---

## Group 2 — Crypto primitives (after T1)

### T5 — Telegram domain newtypes

**Depends on:** T1  
**File:** `lib/http_tools/src/telegram/types.rs` (new file + new `telegram/` module)  
**Action:** Create the module and define:
```rust
pub struct TelegramUserId(pub i64);
pub struct BotToken(SecretString);
pub struct WebhookSecret(SecretString);
pub struct InitData(String);
pub struct AuthDate(u64);
pub struct TelegramUpdateId(u64);

pub struct TelegramUser {
    pub id: TelegramUserId,
    pub username: Option<String>,
}

pub enum TelegramVerifyError {
    MissingHash,
    InvalidHmac,
    Expired,
    MissingUserField,
    MalformedUserField,
}
```

Wire the new module into `lib/http_tools/src/lib.rs`.

**Acceptance:**
- [ ] `cargo build -p myc-http-tools` passes
- [ ] `BotToken` and `WebhookSecret` are `Debug`-redacted (verify via `format!("{:?}", token)` in test — must not print the value)
- [ ] No `#[derive(Clone)]` on `BotToken` or `WebhookSecret` (secrets should not be cheaply cloned)

---

### T6 — `verify_init_data` with unit tests

**Depends on:** T5  
**File:** `lib/http_tools/src/telegram/verify_init_data.rs`  
**Signature:**
```rust
pub fn verify_init_data(
    raw: &InitData,
    bot_token: &BotToken,
    now: DateTime<Utc>,
) -> Result<TelegramUser, TelegramVerifyError>
```

**Algorithm:** exactly as specified in `telegram.md §2.1` — URL-decode, split, remove hash, sort, join with `\n`, HMAC-SHA256 with `WebAppData` secret key, constant-time compare, parse user, check `auth_date`.

**Unit tests (mandatory, cover all branches):**
- Valid `initData` within TTL → `Ok(TelegramUser)`
- Valid HMAC but expired `auth_date` → `Err(Expired)`
- Tampered payload (wrong hash) → `Err(InvalidHmac)`
- Missing `hash` field → `Err(MissingHash)`
- Missing `user` field → `Err(MissingUserField)`
- `username` absent in user JSON → `Ok` with `username: None`

Use test vectors from the Telegram documentation if available; otherwise construct vectors by running the algorithm manually with a known bot token.

**Acceptance:**
- [ ] All 6 test cases pass
- [ ] `cargo test -p myc-http-tools verify_init_data` passes
- [ ] Function is `pub fn` (not async) — pure, no I/O
- [ ] HMAC comparison uses `subtle::ConstantTimeEq`, not `==` on byte slices

---

### T7 — `verify_webhook_secret` with unit tests

**Depends on:** T5  
**File:** `lib/http_tools/src/telegram/verify_webhook_secret.rs`  
**Signature:**
```rust
pub fn verify_webhook_secret(
    header: Option<&str>,
    expected: &WebhookSecret,
) -> bool
```

**Unit tests:**
- Correct header value → `true`
- Wrong header value → `false`
- Missing header (`None`) → `false`
- Empty string header → `false`

**Acceptance:**
- [ ] All 4 test cases pass
- [ ] Comparison uses constant-time equality
- [ ] Function never panics on any input

---

## Group 3 — Domain layer (after T3, T5)

### T8 — `TelegramConfigPort` trait

**Depends on:** T3, T5  
**File:** `core/src/domain/entities/telegram_config.rs` (new)  
**Action:**
```rust
#[async_trait]
pub trait TelegramConfigPort: Send + Sync {
    async fn get_bot_token(
        &self,
        tenant_id: Uuid,
    ) -> Result<BotToken, MappedErrors>;

    async fn get_webhook_secret(
        &self,
        tenant_id: Uuid,
    ) -> Result<WebhookSecret, MappedErrors>;
}
```

Wire into `core/src/domain/entities/mod.rs`.

**Note:** `BotToken` and `WebhookSecret` are from `myc-http-tools`. The trait lives in `core` but imports the newtypes from `lib/http_tools` via the existing dependency path (check `core/Cargo.toml`).

**Acceptance:**
- [ ] `cargo build -p myc-core` passes
- [ ] Trait is object-safe (no generic methods)
- [ ] No adapter or concrete type imported into `core`

---

### T9 — `IdentitySource` enum + `Route` DTO extension

**Depends on:** nothing (pure domain DTO change)  
**File:** `core/src/domain/dtos/route.rs`  
**Action:** Add before the `Route` struct:
```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum IdentitySource {
    Telegram,
}
```
Add field to `Route`:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub identity_source: Option<IdentitySource>,
```
Update `Route::new(...)` constructor signature and all call sites. Update existing `create_test_route` helpers in `#[cfg(test)]` to pass `None`.

**Acceptance:**
- [ ] `cargo test -p myc-core` passes (all existing route tests still pass)
- [ ] `identity_source: None` serializes without the key in JSON (verified in a test)
- [ ] `identity_source: Some(IdentitySource::Telegram)` round-trips through serde correctly

---

### T10 — `resolve_account_by_telegram_id` use-case

**Depends on:** T8  
**File:** `core/src/use_cases/gateway/telegram/resolve.rs` (new, plus `mod.rs` for the module)  
**Signature:**
```rust
pub async fn resolve_account_by_telegram_id(
    telegram_user_id: TelegramUserId,
    tenant_id: Uuid,
    account_meta_repo: Box<&dyn AccountMetaFetchingPort>,
) -> Result<Account, MappedErrors>
```

**Logic:**
1. Query `account.meta` for `telegram_user.id == telegram_user_id` AND `tenant_id == tenant_id`.
2. If not found → `fetching_err("telegram_id_not_linked").with_exp_true()`.
3. Return the resolved `Account`.

**Acceptance:**
- [ ] Unit test: found case → returns `Ok(Account)`
- [ ] Unit test: not found → returns `Err` with `exp = true`
- [ ] No concrete adapter imported; uses trait object only
- [ ] `tenant_id` is always passed — no lookup without tenant scope (see spec invariant OQ-2b)

---

## Group 4 — Use-cases (after T6, T8, T10)

### T11 — `link_telegram_identity` use-case

**Depends on:** T6, T8, T10  
**File:** `core/src/use_cases/gateway/telegram/link.rs`

**Signature:**
```rust
pub async fn link_telegram_identity(
    account_id: Uuid,
    tenant_id: Uuid,
    init_data: InitData,
    telegram_config: Box<&dyn TelegramConfigPort>,
    account_meta_repo: Box<&dyn AccountMetaRepo>,
    audit_repo: Box<&dyn TelegramAuditPort>,
    now: DateTime<Utc>,
) -> Result<(), MappedErrors>
```

**Steps (from spec §6.1, excluding JWT validation which happens in the handler):**
1. Load `BotToken` via `telegram_config.get_bot_token(tenant_id)`.
2. Call `verify_init_data(&init_data, &bot_token, now)`. Map error to `use_case_err`.
3. Extract `TelegramUserId`.
4. Check if account already has `telegram_user` meta → `use_case_err("already_linked").with_exp_true()`.
5. Call `resolve_account_by_telegram_id` to check cross-account conflict → if found → `use_case_err("telegram_id_already_used").with_exp_true()`.
6. Write `telegram_user: { id, username }` to `account.meta`.
7. Write audit: `event=linked`.

**Acceptance:**
- [ ] Unit tests for: success, already_linked, telegram_id_already_used, invalid_init_data, expired_init_data
- [ ] Mocked trait objects via `mockall`
- [ ] `bot_token` excluded from tracing span

---

### T12 — `unlink_telegram_identity` use-case

**Depends on:** T8  
**File:** `core/src/use_cases/gateway/telegram/unlink.rs`

**Steps (from spec §6.2):**
1. Verify account has `telegram_user` meta → if absent → `fetching_err("not_linked").with_exp_true()`.
2. Remove `telegram_user` from `account.meta`.
3. Write audit: `event=unlinked`.

**Acceptance:**
- [ ] Unit tests: success, not_linked

---

### T13 — `login_via_telegram` use-case

**Depends on:** T6, T8, T10  
**File:** `core/src/use_cases/gateway/telegram/login.rs`

**Steps (from spec §6.3):**
1. Load `BotToken` via `telegram_config`.
2. `verify_init_data(...)`. On error → write audit `event=login_fail`, return `use_case_err`.
3. Extract `TelegramUserId`.
4. `resolve_account_by_telegram_id(telegram_user_id, tenant_id, ...)`. On error → write `login_fail`, return err.
5. Build `UserAccountScope` connection string via `UserAccountScope::new(account_id, expires_at, roles, tenant_id, ...)`.
6. Write audit: `event=login_ok`.
7. Return `(UserAccountScope, expires_at)`.

**Acceptance:**
- [ ] Unit tests: success → connection string returned, invalid_init_data → login_fail audited, not_linked → login_fail audited
- [ ] `bot_token` excluded from tracing span
- [ ] Connection string is not logged

---

## Group 5 — Adapter (after T3, T8)

### T14 — `TelegramConfigAdapter`

**Depends on:** T3, T8  
**File:** `adapters/service/src/telegram_config.rs` (new)

**Action:** Implement `TelegramConfigPort` for a struct that:
1. Receives a reference to the `TenantMetaRepo` (to read `TelegramBotTokenRef` and `TelegramWebhookSecretRef` from `tenant.meta`).
2. Reads the Vault path string stored in the meta key.
3. Resolves the secret via `SecretResolver::VaultSecret(path).async_get_or_error().await`.
4. Returns `BotToken(SecretString::new(value))` or `WebhookSecret(...)`.

**Acceptance:**
- [ ] `cargo build -p myc-service-adapter` passes (or equivalent adapter crate name)
- [ ] Integration test with a mock `TenantMetaRepo` returning a known Vault path → correct `BotToken` returned
- [ ] If Vault path is absent from meta → `fetching_err("telegram_not_configured").with_exp_true()`

---

## Group 6 — HTTP handlers (after T11–T14, T9)

### T15 — `POST /auth/telegram/link` handler

**Depends on:** T11, T14  
**File:** `ports/api/src/rest/gateway/telegram/link.rs`

**Handler responsibilities (JWT validation happens here, not in use-case):**
1. Extract and verify Mycelium JWT → `account_id`, `session_tenant_id`.
2. Deserialize request body: `{ tenant_id: Uuid, init_data: String }`.
3. Assert `body.tenant_id == session_tenant_id` → 403 if mismatch.
4. Rate-limit: 10 req/min per `account_id` (use existing rate-limit middleware or KV counter).
5. Call `link_telegram_identity(...)`.
6. Map use-case errors to HTTP codes per spec §6.1 error taxonomy.
7. Return 204.

**Acceptance:**
- [ ] `cargo build` passes
- [ ] Returns 403 on tenant mismatch (test with mismatched UUIDs)
- [ ] Returns 401 on invalid `initData`
- [ ] Returns 409 on double-link attempt

---

### T16 — `POST /auth/telegram/unlink` handler

**Depends on:** T12, T14  
**File:** `ports/api/src/rest/gateway/telegram/unlink.rs`

**Acceptance:**
- [ ] Returns 404 when account has no Telegram link
- [ ] Returns 204 on success
- [ ] Returns 403 on tenant mismatch

---

### T17 — `POST /auth/telegram/login` handler

**Depends on:** T13, T14  
**File:** `ports/api/src/rest/gateway/telegram/login.rs`

**Rate-limit:** 20 req / 5 min per (`tenant_id` + hashed `from.id`). Applied before calling the use-case. `from.id` is not yet known at this stage — rate-limit on `tenant_id` alone at handler entry; a secondary per-`from.id` limit can be applied inside the use-case after HMAC verification.

**Response:**
```json
{ "connection_string": "...", "expires_at": "..." }
```

**Acceptance:**
- [ ] Returns 401 on invalid or expired `initData`
- [ ] Returns 401 on unlinked `from.id` (same HTTP code — do not distinguish)
- [ ] Connection string is present and non-empty in success response
- [ ] `expires_at` is a valid RFC 3339 timestamp

---

### T18 — `POST /webhooks/telegram/{tenant_id}` handler

**Depends on:** T7, T10, T14  
**File:** `ports/api/src/rest/gateway/telegram/webhook.rs`

This is the most complex handler. Follow the 13-step sequence from spec §6.4 exactly.

Key implementation notes:
- Body size check must happen **before** deserialization (use `web::Bytes` extractor with a size limit, not `web::Json`).
- After step 7 (KV write), downstream forwarding is **fire-and-forget** — spawn via `actix_web::rt::spawn` or use the existing async callback engine. Do not `await` the downstream call before returning 200.
- Always return `HttpResponse::Ok().finish()` regardless of internal outcome. Log errors; never surface them to Telegram.

**Acceptance:**
- [ ] Returns 200 on all paths (verified with: missing secret, wrong secret, unknown tenant, unlinked user, valid user)
- [ ] Returns 413 on body > 512 KB
- [ ] Duplicate `update_id` within 24h does not forward to downstream (verified via KV mock)
- [ ] Valid message from linked user: profile headers present in forwarded request

---

### T19 — `check_security_group` branch for `identity_source: Telegram` (Mode B)

**Depends on:** T9, T10, T14  
**File:** `ports/api/src/router/check_security_group.rs`

**Action:** Add a guard **before** the existing `security_group` match:
```
if let Some(IdentitySource::Telegram) = route.identity_source {
    // 1. source reliability check (IP allowlist) — mandatory, non-bypassable
    // 2. extract from.id from request body JSON
    // 3. extract tenant_id from route context or x-mycelium-tenant-id header
    // 4. resolve_account_by_telegram_id(from_id, tenant_id)
    // 5. build Profile, inject headers
    // 6. return (downstream_request, security_group, Some(UserInfo::new_profile(profile)))
}
```

If source reliability check fails → return `GatewayError::Forbidden`.  
If `from.id` not found in body → return `GatewayError::Unauthorized`.  
If account not linked → return `GatewayError::Unauthorized`.

**Acceptance:**
- [ ] Existing security group tests all pass unchanged
- [ ] New test: route with `identity_source: Telegram` + n8n IP in allowlist + linked user → profile injected
- [ ] New test: route with `identity_source: Telegram` + IP NOT in allowlist → 403
- [ ] New test: route with `identity_source: Telegram` + unlinked `from.id` → 401

---

## Group 7 — DI wiring and route registration (after T15–T19)

### T20 — Wire `TelegramConfigAdapter` via shaku

**Depends on:** T14, T15–T18  
**File:** `ports/api/src/` (wherever shaku modules are registered — check existing pattern)  
**Action:** Register `TelegramConfigAdapter` as the implementation of `TelegramConfigPort` in the shaku module. Inject into all four handlers.

**Acceptance:**
- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace` passes

---

### T21 — Register routes in actix-web app

**Depends on:** T20  
**File:** `ports/api/src/` (main router config)  
**Action:** Register:
```
POST /auth/telegram/link
POST /auth/telegram/unlink
POST /auth/telegram/login
POST /webhooks/telegram/{tenant_id}
```

**Acceptance:**
- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace --all` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] All four routes return expected HTTP codes via a basic integration smoke test

---

## Dependency graph (summary)

```
T0
└── T1
    ├── T5
    │   ├── T6 ──────────────────┐
    │   │                        ├── T11 ── T15
    │   ├── T7 ── T18            ├── T13 ── T17
    │   └── T8 ──────────────────┤
    │       └── T10 ─────────────┘
    │                            └── T12 ── T16
T2 (independent)
T3 ──── T8, T14 ── T19, T20
T4 (verification gate)
T9 ── T19

All T15–T19 ── T20 ── T21
```

---

## Implementation order (sequential, safe path)

1. T0 → T1, T2, T3, T4 (parallel)
2. T5 → T6, T7 (parallel)
3. T8, T9 (parallel, after T3/T5)
4. T10 (after T8)
5. T11, T12, T13 (parallel, after T10/T6)
6. T14 (after T3/T8)
7. T15, T16, T17, T18, T19 (parallel, after their deps)
8. T20 → T21
