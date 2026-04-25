# Tasks: Magic Link Auth (Gateway)

Legend: ⬜ not started · 🔄 in progress · ✅ done · 🔴 blocked

---

## GT0 — `MagicLinkTokenMeta` DTO + `MultiTypeMeta` variant

**Status:** ✅

**What:**
1. Add `MagicLinkTokenMeta { email: Email, token: Option<String>, code: String }` to
   `core/src/domain/dtos/token/meta.rs`
2. `MagicLinkTokenMeta::new(email)` generates: `token = Some(Uuid::new_v4().to_string())`,
   `code = format!("{:06}", rand_u32 % 1_000_000)`
3. Add `MultiTypeMeta::MagicLink(MagicLinkTokenMeta)` to the enum in
   `core/src/domain/dtos/token/mod.rs`
4. Add unit test verifying serialization round-trip of the new variant

**Spec:** ML-001, ML-004

**Done when:** `cargo build -p myc-core` passes, round-trip test passes

---

## GT1 — Port methods

**Status:** ✅

**Depends on:** GT0

**What:**
1. `core/src/domain/entities/token/token_registration.rs` — add:
   ```rust
   async fn create_magic_link_token(
       &self,
       meta: MagicLinkTokenMeta,
       expires: DateTime<Local>,
   ) -> Result<CreateResponseKind<Token>, MappedErrors>;
   ```
2. `core/src/domain/entities/token/token_invalidation.rs` — add:
   ```rust
   async fn get_code_and_invalidate_display_token(
       &self,
       email: &Email,
       token: &str,
   ) -> Result<FetchResponseKind<String, String>, MappedErrors>;

   async fn get_and_invalidate_magic_link_code(
       &self,
       email: &Email,
       code: &str,
   ) -> Result<FetchResponseKind<(), String>, MappedErrors>;
   ```

**Spec:** ML-001, ML-002, ML-003, ML-004

**Done when:** `cargo build -p myc-core` passes

---

## GT2 — Diesel adapter: registration

**Status:** ✅

**Depends on:** GT1

**What:**
`adapters/diesel/src/repositories/token/token_registration.rs`

Implement `create_magic_link_token`:
- Same INSERT pattern as `create_password_change_token`
- `meta_value = to_value(meta)?` (no encryption needed — token is UUID, code is not secret
  once displayed; the DB is the source of truth for validity)
- Return `CreateResponseKind::Created(Token::new(...))`

**Spec:** ML-001

**Done when:** `cargo build --workspace` passes

---

## GT3 — Diesel adapter: invalidation

**Status:** ✅

**Depends on:** GT1

**What:**
`adapters/diesel/src/repositories/token/token_invalidation.rs`

Implement `get_code_and_invalidate_display_token(email, token)`:
- SELECT token record WHERE `meta->'MagicLink'->>'token' = ?` AND `meta->'MagicLink'->>'email' = ?`
  AND expiration > now()
- If not found → return `FetchResponseKind::NotFound(...)`
- If found → UPDATE set `meta['MagicLink']['token'] = null` (consume display token)
- Return `FetchResponseKind::Found(code)` where code = `meta['MagicLink']['code']`

Implement `get_and_invalidate_magic_link_code(email, code)`:
- SELECT WHERE `meta->'MagicLink'->>'code' = ?` AND `meta->'MagicLink'->>'email' = ?`
  AND `meta->'MagicLink'->>'token' IS NULL` (token already consumed)
  AND expiration > now()
- If not found → `FetchResponseKind::NotFound(...)`
- If found → DELETE record
- Return `FetchResponseKind::Found(())`

Note: requiring `token IS NULL` on verify enforces that the display page was opened before
verify is attempted. If the display was never opened (token not consumed), verify fails.

**Spec:** ML-003, ML-004

**Done when:** `cargo build --workspace` passes

---

## GT4 — Use case: `request_magic_link`

**Status:** ✅

**Depends on:** GT1

**What:**
Create `core/src/use_cases/role_scoped/beginner/user/request_magic_link.rs`:

```
pub async fn request_magic_link(
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<(), MappedErrors>
```

Steps:
1. Build `meta = MagicLinkTokenMeta::new(email.clone())`
2. `expires = Local::now() + Duration::seconds(life_cycle_settings.token_expiration.async_get_or_error().await?)`
3. `token_registration_repo.create_magic_link_token(meta.clone(), expires).await?`
4. Build `display_url`:
   ```
   {domain_url}/_adm/beginners/users/magic-link/display
     ?token={meta.token.unwrap()}&email={urlencoded(email.email())}
   ```
   Use `life_cycle_settings.domain_url` as base.
5. `dispatch_notification(vec![("magic_link_url", display_url)], "email/magic-link-request", ...)`
6. Return `Ok(())`

Register in `core/src/use_cases/role_scoped/beginner/user/mod.rs`.

**Spec:** ML-001

**Done when:** `cargo build -p myc-core` passes; no `.unwrap()` in production path

---

## GT5 — Use case: `verify_magic_link`

**Status:** ✅

**Depends on:** GT1

**What:**
Create `core/src/use_cases/role_scoped/beginner/user/verify_magic_link.rs`:

```
pub async fn verify_magic_link(
    email: Email,
    code: String,
    auth_config: InternalOauthConfig,
    core_config: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
    token_invalidation_repo: Box<&dyn TokenInvalidation>,
) -> Result<(String, Duration), MappedErrors>
```

Steps:
1. `token_invalidation_repo.get_and_invalidate_magic_link_code(&email, &code).await?`
   - If `NotFound` → `use_case_err("Invalid or expired code").with_exp_true().as_error()`
2. Fetch user: `user_fetching_repo.get_user_by_email(email.clone()).await?`
3. If `NotFound`:
   - Build minimal User: active, no password, `Provider::Internal(PasswordHash::hash_user_password(Uuid::new_v4().to_string().as_bytes()))`
   - `user_registration_repo.register_user(user).await?`
   - Use the created user
4. `encode_jwt(user, auth_config, core_config, false).await`
   - On error → propagate as `MappedErrors`
5. Return `Ok((token_string, duration))`

Register in `mod.rs`.

**Spec:** ML-003

**Done when:** `cargo build -p myc-core` passes; no `.unwrap()` in production path

---

## GT6 — REST endpoints + templates

**Status:** ✅

**Depends on:** GT2, GT3, GT4, GT5

**What:**

In `ports/api/src/rest/role_scoped/beginners/user_endpoints.rs`:

**Structs:**
```rust
struct MagicLinkRequestBody { email: String }
struct MagicLinkRequestResponse { sent: bool }
struct MagicLinkDisplayParams { token: String, email: String }
struct MagicLinkVerifyBody { email: String, code: String }
```

**Handlers:**

`#[post("/magic-link/request")] request_magic_link_url`:
- No auth extractor
- Call `request_magic_link(...)`, always return `200 { "sent": true }`

`#[get("/magic-link/display")] display_magic_link_url`:
- No auth extractor
- Query params: `MagicLinkDisplayParams`
- Parse email, call `get_code_and_invalidate_display_token(&email, &params.token)`
- If `NotFound` → render Tera template `"web/magic-link-display-error"` with HTTP 401
- If `Found(code)` → render Tera template `"web/magic-link-display"` with `{ code, app_name, expires_in_minutes: 15 }`
- Return `text/html`

`#[post("/magic-link/verify")] verify_magic_link_url`:
- No auth extractor
- Parse email, call `verify_magic_link(...)`
- If error (expected) → `401 Unauthorized`
- If ok → `200 MyceliumLoginResponse { token, duration, totp_required: false, user }`

Register all three in `configure()`.
Add `security(())` to all utoipa annotations.

**Templates** (add to existing templates directory):
- `email/magic-link-request.html` — variables: `magic_link_url`, `app_name`
- `web/magic-link-display.html` — variables: `code`, `app_name`, `expires_in_minutes`
- `web/magic-link-display-error.html` — variables: `app_name`

**Spec:** ML-001, ML-002, ML-003

**Gate:**
```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all
```

---

## GT7 — RPC dispatcher fix

**Status:** ✅

**What:**
`ports/api/src/rpc/dispatchers/beginners.rs` — `BEGINNERS_ACCOUNTS_CREATE` arm:

Replace:
```rust
} else {
    return Err(invalid_params("Invalid provider"));
};
```
With:
```rust
} else {
    MYCELIUM_PROVIDER_KEY.to_string()
};
```

Add `use myc_http_tools::settings::MYCELIUM_PROVIDER_KEY;` if not already imported.

**Spec:** ML-005

**Gate:**
```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all
```
