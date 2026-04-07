# Feature Spec: Magic Link (Passwordless Login)

**Milestone:** M3 — Auth Evolution
**Scope:** Gateway (Rust)
**Status:** Specified

---

## Context

The gateway already has:
- Internal JWT provider (`iss: "mycelium"`, HS512, `encode_jwt`)
- Token infrastructure: `MultiTypeMeta` enum + `TokenRegistration` / `TokenInvalidation` traits
  stored as JSONB in the `token` table
- Notification infrastructure: `dispatch_notification` + Tera email templates
- Password-change flow as reference pattern (`start_password_redefinition` + `check_token_and_reset_password`)

What is missing:
- Magic link request / display / verify use cases
- `MagicLinkTokenMeta` DTO with `token` (UUID) + `code` (6-digit)
- Two-phase token consumption: token consumed on display, code consumed on verify
- A gateway-rendered HTML page (Tera) that shows the code
- RPC dispatcher fix: `BEGINNERS_ACCOUNTS_CREATE` rejects internal provider — must be fixed

---

## Requirements

### ML-001 — Request magic link

Public endpoint (no auth). Accepts an email address, generates a pair
`(token: UUID, code: 6-digit string)`, stores them together with a 15-minute TTL, and
sends an email containing a link to the display page.

**Endpoint:** `POST /_adm/beginners/users/magic-link/request`
**Auth:** none
**Body:** `{ "email": "user@example.com" }`
**Response:** `200 { "sent": true }` — always, regardless of whether the email is registered
(prevents user enumeration)

The email contains a link of the form:
```
{domain_url}/_adm/beginners/users/magic-link/display?token=<uuid>&email=<encoded-email>
```

### ML-002 — Display page (gateway-rendered HTML)

Public endpoint (no auth). Accepts `token` + `email` as query parameters.

1. Fetches the record `{ email, token, code }` by `(email, token)` from the DB
2. If not found or expired → renders an error HTML page ("link inválido ou expirado")
3. If found → **invalidates the token** (single-use: this link cannot be opened again)
   while keeping the code valid in the DB for the verify step
4. Renders an HTML page (Tera template `magic-link-display`) that shows the 6-digit code

**Endpoint:** `GET /_adm/beginners/users/magic-link/display`
**Auth:** none
**Query params:** `token`, `email`
**Response:** `200 text/html` — rendered Tera template

The token is single-use. Opening the link twice renders the error page on the second visit.

### ML-003 — Verify code and issue JWT

Public endpoint (no auth). Accepts `email` + `code`. Validates the pair, issues a JWT.

1. Fetches record by `(email, code)` from the DB
2. If not found or expired → `401 Unauthorized`
3. **Deletes the record** (code is single-use)
4. If no `User` record exists for `email` → creates a minimal active User
   (no password, provider = internal sentinel, `is_active = true`)
5. Issues JWT via `encode_jwt` (HS512, `iss: "mycelium"`, same as `/login`)
6. Returns `200 MyceliumLoginResponse { token, duration, totpRequired: false, ...user }`

**Endpoint:** `POST /_adm/beginners/users/magic-link/verify`
**Auth:** none
**Body:** `{ "email": "user@example.com", "code": "847291" }`
**Response:** `200 { token, duration, totpRequired, ...User }`

### ML-004 — Two-phase token consumption

The stored record has two independent secrets: `token` (UUID, in the email link) and
`code` (6-digit, shown on the display page).

- **Phase 1** (display): the `token` is consumed. The link becomes invalid, but `code`
  remains available for the verify step.
- **Phase 2** (verify): the `code` + `email` pair is consumed. The record is deleted.

Implementation approach: store one JSONB record `MagicLinkTokenMeta { email, token: Option<String>, code: String }`.
- Display step: fetch by `(email, token)`, update `token` to `None` (or a consumed sentinel), return `code`
- Verify step: fetch by `(email, code)`, delete record, proceed to JWT

### ML-005 — RPC dispatcher fix: `BEGINNERS_ACCOUNTS_CREATE` for internal provider

`ports/api/src/rpc/dispatchers/beginners.rs` — method `BEGINNERS_ACCOUNTS_CREATE`
currently returns `Err(invalid_params("Invalid provider"))` when `external_provider` is `None`
(i.e., user authenticated via the internal Mycelium provider).

Fix: when `external_provider` is `None`, use `MYCELIUM_PROVIDER_KEY` as the issuer string
and proceed. The use case `create_user_account` already accepts `Option<String>` for provider.

This unblocks magic-link-authenticated users from creating their Mycelium account on first login.

---

## DTO: `MagicLinkTokenMeta`

```rust
// core/src/domain/dtos/token/meta.rs (or new magic_link.rs)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MagicLinkTokenMeta {
    pub email: Email,
    /// UUID placed in the email link. Set to None after display page is opened.
    pub token: Option<String>,
    /// 6-digit code shown on the display page. Consumed on verify.
    pub code: String,
}

impl MagicLinkTokenMeta {
    pub fn new(email: Email) -> Self {
        Self {
            email,
            token: Some(Uuid::new_v4().to_string()),
            code: format!("{:06}", rand::random::<u32>() % 1_000_000),
        }
    }
}
```

Add `MultiTypeMeta::MagicLink(MagicLinkTokenMeta)` to the enum.

---

## New port methods

### `TokenRegistration` (new method)
```rust
async fn create_magic_link_token(
    &self,
    meta: MagicLinkTokenMeta,
    expires: DateTime<Local>,
) -> Result<CreateResponseKind<Token>, MappedErrors>;
```

### `TokenInvalidation` (two new methods)
```rust
/// Phase 1 — consume display token, return code for rendering
async fn get_code_and_invalidate_display_token(
    &self,
    email: &Email,
    token: &str,
) -> Result<FetchResponseKind<String, String>, MappedErrors>;

/// Phase 2 — consume code, delete record
async fn get_and_invalidate_magic_link_code(
    &self,
    email: &Email,
    code: &str,
) -> Result<FetchResponseKind<(), String>, MappedErrors>;
```

---

## New use cases

```
core/src/use_cases/role_scoped/beginner/user/request_magic_link.rs
core/src/use_cases/role_scoped/beginner/user/verify_magic_link.rs
```

The display page is handled directly in the REST handler (not a use case), since it is
purely a token lookup + template render with no domain logic.

---

## New REST endpoints

Added to `ports/api/src/rest/role_scoped/beginners/user_endpoints.rs`:

| Method | Path | Handler |
|---|---|---|
| POST | `/magic-link/request` | `request_magic_link_url` |
| GET | `/magic-link/display` | `display_magic_link_url` |
| POST | `/magic-link/verify` | `verify_magic_link_url` |

All three added to `configure()`. All three annotated with `security(())` (public).

The display endpoint returns `text/html` (Tera-rendered), not JSON.

---

## Email template

Template name: `"email/magic-link-request"`
Variables: `{{ magic_link_url }}`, `{{ app_name }}`

---

## Display page template

Template name: `"web/magic-link-display"`
Variables: `{{ code }}`, `{{ app_name }}`, `{{ expires_in_minutes }}`

Renders a minimal HTML page showing the code and instructing the user to enter it in the
app. Includes a note that the code expires.

---

## Out of scope

- TOTP 2FA for magic link users (`totpRequired` is always `false`)
- Rate limiting (M4)
- Telegram / WhatsApp providers (separate M3 sub-feature)
- Standalone mode (separate M3 sub-feature)
