# Telegram Identity Provider — Implementation Spec

**Status:** Spec — ready for task breakdown  
**Date:** 2026-04-19  
**Last updated:** 2026-04-19 (OQ-1 resolved; authentication modes documented)  
**Scope:** Telegram only. WhatsApp is a separate spec.

---

## 1. Threat model

### Adversaries and invariants

| Adversary | Attack vector | Invariant that must hold |
|---|---|---|
| Replayed `initData` | Reuse valid `initData` after TTL | `auth_date` must be within ±300 s of server time. Reject outside window. |
| Forged `initData` | Attacker self-declares `from.id` | HMAC-SHA256 over sorted key=value pairs using `HMAC(SHA256(bot_token), "WebAppData")` as key. Constant-time comparison. |
| Mutable username | Username changes between link and lookup | `from.id` is the canonical identity. `from.username` is stored for display only. Lookup and linking always use `from.id`. |
| Stolen bot token | Attacker forges any `initData` | Bot token lives only in Vault. Never logged, never serialized into response. Explicitly excluded in all `#[tracing::instrument(skip(...))]`. |
| Cross-tenant link | Attacker supplies a different `tenant_id` in URL | Linking requires an active authenticated Mycelium session (JWT). The session's tenant must match the URL `tenant_id`. |
| Double-link | Two accounts link the same Telegram `from.id` | Unique functional index on `account.meta` JSONB for `telegram_user.id` — global scope. Reject with 409 at DB level, surface as a domain error. |
| Account enumeration via webhook | Probe `tenant_id` values to discover valid tenants | Rate-limit per `tenant_id` before DB lookup. Respond with 200 OK to Telegram regardless of resolution outcome (Telegram requires 200 within 5 s). |
| Compromised n8n (Leg 2) | n8n sends forged body with arbitrary `from.id` | Routes that accept `identity_source: Telegram` must have source reliability (IP allowlist) enforced. Body is not trusted as the sole control: n8n must be in the allowlist AND the `from.id` must resolve to an existing linked account. |
| Oversized webhook body | DoS via huge request | Body size cap: 512 KB. Enforced at the endpoint extractor before any parsing. |
| Replay via `update_id` | Re-deliver webhook to trigger downstream twice | Track `update_id` in Redis (KV) with a 24 h TTL. Idempotent: duplicate `update_id` returns 200 but does not forward to downstream. |

---

## 1b. Authentication modes

The Telegram IdP supports two coexisting authentication modes. Operators choose per route/use-case — they are not mutually exclusive.

### Mode A — Token exchange (agents: MCP / REST)

Used when an AI agent or any service needs to call Mycelium APIs **authenticated as the Telegram user**.

```
Telegram Mini App → agent receives initData
                        ↓
              POST /auth/telegram/login
              { tenant_id, init_data }
                        ↓
            Mycelium validates HMAC, resolves linked account
                        ↓
            Returns a UserAccountScope connection string
                        ↓
Agent calls GET /mcp/... or POST /rpc/... with the connection string
Mycelium accepts it — same path as Bearer token auth
```

The connection string is a `UserAccountScope` — an HMAC-signed, Base64-encoded token already used throughout the system (`core/src/domain/dtos/token/connection_string/`). Mycelium accepts both Bearer token (Auth0 JWT) and connection string as authentication mechanisms. No new token infrastructure required — Telegram login plugs directly into the existing connection-string auth path.

**Who uses this:** AI agents (Claude, n8n with token mode, any HTTP client) that receive `initData` from a Mini App and need to act on behalf of the user.

### Mode B — Body passthrough (n8n Leg 2)

Used when a downstream (e.g. n8n) receives a forwarded Telegram update from Mycelium and needs to call other Mycelium-protected services, passing the original Telegram body so Mycelium resolves the identity inline.

```
Telegram → Mycelium webhook → n8n (with user profile injected)
                                  ↓
           n8n calls POST /some-protected-route
           forwarding the original Telegram body
                                  ↓
           Mycelium checks: route has identity_source: Telegram
           + n8n IP is in allowlist
           + extracts from.id from body
           + resolves account → builds profile
           + injects profile headers → routes to downstream
```

**Who uses this:** n8n workflows that were triggered by a Telegram webhook and need to call further Mycelium-protected endpoints without performing a token exchange first.

### Comparison

| | Mode A | Mode B |
|---|---|---|
| Auth mechanism | Connection string token | Body passthrough + IP allowlist |
| Requires token exchange | Yes (`/auth/telegram/login`) | No |
| Route config | Standard (`Protected`) | `identity_source: Telegram` + allowlist |
| Best for | AI agents, MCP calls | n8n multi-step workflows |
| initData required by caller | Yes | No (uses original Telegram body) |

---

## 2. Cryptographic protocols

### 2.1 initData verification (Mini App linking and login)

Algorithm defined at: https://core.telegram.org/bots/webapps#validating-data-received-via-the-mini-app

**Steps (in this exact order):**

1. URL-decode the raw `initData` string.
2. Split into key=value pairs by `&`.
3. Extract and remove the `hash` field. If absent → reject with 400.
4. Sort remaining pairs lexicographically by key.
5. Join sorted pairs with `\n` as separator.
6. Compute `secret_key = HMAC-SHA256(key=b"WebAppData", data=bot_token_bytes)`.
7. Compute `expected_hash = HMAC-SHA256(key=secret_key, data=data_check_string).to_hex()`.
8. Compare `expected_hash` == `hash` using **constant-time** equality (`subtle::ConstantTimeEq` or `constant_time_eq` crate).
9. If mismatch → reject with 401.
10. Parse `user` JSON from the `user` field. Extract `from.id` (i64) and `from.username` (String, optional).
11. Parse `auth_date` (u64 Unix timestamp). Compute `|now_utc - auth_date|`. If > `AUTH_DATE_MAX_AGE_SECS` → reject with 401.

**Constants:**

```
AUTH_DATE_MAX_AGE_SECS = 300   // 5 minutes; same window for link and login
```

**Test vectors:** use the vectors from the Telegram documentation in unit tests covering: valid case, wrong hash, expired `auth_date`, missing hash, missing user field.

**Location:** `lib/http_tools/src/telegram/verify_init_data.rs`  
**Signature:** `pub fn verify_init_data(raw: &InitData, bot_token: &BotToken, now: DateTime<Utc>) -> Result<TelegramUser, TelegramVerifyError>`  
Accepting `now` as a parameter makes the function purely testable without mocking the clock.

### 2.2 Webhook `X-Telegram-Bot-Api-Secret-Token` check

Telegram sends the header value set during `setWebhook`. Stored in Vault per tenant.

- Compare header value against stored `webhook_secret` using **constant-time** equality.
- If missing or mismatch → return 200 OK (do not reveal rejection reason to Telegram).
- Log the rejection internally at `WARN` level with `tenant_id` but without the token value.

**Location:** `lib/http_tools/src/telegram/verify_webhook_secret.rs`  
**Signature:** `pub fn verify_webhook_secret(header: Option<&str>, expected: &WebhookSecret) -> bool`

---

## 3. Newtypes

All cross-boundary primitives carry domain newtypes. No raw `String` or `i64` across function signatures.

```rust
// lib/http_tools/src/telegram/types.rs (or core/src/domain/dtos/telegram.rs)

pub struct TelegramUserId(pub i64);
pub struct BotToken(SecretString);         // zeroize on drop
pub struct WebhookSecret(SecretString);    // zeroize on drop
pub struct InitData(String);               // raw URL-encoded string
pub struct AuthDate(u64);                  // Unix timestamp
pub struct TelegramUpdateId(u64);

pub struct TelegramUser {
    pub id: TelegramUserId,
    pub username: Option<String>,          // display only, never for lookup
}
```

`SecretString` from the `secrecy` crate ensures the value is zeroized and is `Debug`-redacted.

---

## 4. Per-tenant secrets

**Problem:** The existing `SecretResolver` is bound to `Service.secrets` (a service-level concept). Bot tokens and webhook secrets are per-tenant.

**Decision:** Add two new `TenantMetaKey` variants:

```rust
TelegramBotTokenRef,    // value: Vault path string, e.g. "secret/tenants/{id}/telegram/bot_token"
TelegramWebhookSecretRef, // value: Vault path string
```

The values stored in `tenant.meta` JSONB are **Vault path references**, not the secrets themselves. At runtime, a `TelegramConfigPort` trait (in `core/domain/entities/`) resolves them via the existing `SecretResolver::VaultSecret` mechanism.

**Invariant:** `bot_token` and `webhook_secret` values must never appear in:
- `tenant.meta` JSONB
- Logs (excluded from all tracing spans)
- API responses
- Error messages returned to the client

---

## 5. Data model and migrations

### 5.1 GIN index on `account.meta`

```sql
-- adapters/diesel/sql/XXXX_telegram_gin_index.sql
CREATE INDEX CONCURRENTLY IF NOT EXISTS
    idx_account_meta_gin
ON account USING GIN (meta jsonb_path_ops);
```

Run `CONCURRENTLY` — zero downtime on existing tables.

### 5.2 Unique functional index for Telegram identity (global)

Prevents double-linking the same `from.id` to more than one account. Telegram identity links to a **personal account** (user/manager/staff type), which has no `tenant_id` column — the constraint must therefore be global, not per-tenant.

```sql
DROP INDEX CONCURRENTLY IF EXISTS idx_account_meta_telegram_user_id_per_tenant;
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS
    idx_account_meta_telegram_user_id_global
ON account ((meta -> 'telegram_user' ->> 'id'))
WHERE meta ? 'telegram_user';
```

**Invariant enforced:** `from.id` is globally unique across all accounts.

### 5.3 Audit table

```sql
CREATE TABLE telegram_identity_audit (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL,
    account_id  UUID NOT NULL,
    event       TEXT NOT NULL CHECK (event IN ('linked', 'unlinked', 'login_ok', 'login_fail')),
    telegram_id BIGINT,          -- stored for audit; NULL on unlinked
    ip          INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON telegram_identity_audit (tenant_id, account_id);
CREATE INDEX ON telegram_identity_audit (created_at);
```

### 5.4 Stored `TelegramUser` shape in `account.meta`

Key: `telegram_user` (serialized `AccountMetaKey::TelegramUser`)

```json
{
  "id": 123456789,
  "username": "john_doe"
}
```

`username` is nullable. `id` is the only lookup key.

---

## 6. Endpoint specifications

### 6.1 `POST /auth/telegram/link`

**Purpose:** Link an authenticated Mycelium account to a Telegram identity.  
**Auth required:** Yes — valid Mycelium JWT (the account holder must be authenticated).

**Request:**
```json
{
  "tenant_id": "uuid",
  "init_data": "<raw URL-encoded initData from Mini App>"
}
```

**Validation steps (in order):**

1. Verify Mycelium JWT → extract `account_id` and `tenant_id` from profile.
2. Assert `request.tenant_id == session.tenant_id`. If mismatch → 403.
3. Load `BotToken` for `tenant_id` via `TelegramConfigPort`.
4. Call `verify_init_data(init_data, bot_token, Utc::now())`. If error → 401.
5. Extract `TelegramUserId` from verified result.
6. Check if this account already has a `telegram_user` meta key. If yes → 409 (`already_linked`).
7. Check if `TelegramUserId` is already linked to any account globally (DB query). If yes → 409 (`telegram_id_already_used`).
8. Write `telegram_user: { id, username }` to `account.meta`.
9. Write audit record: `event=linked`.
10. Return 204.

**Rate limit:** 10 requests / minute per `account_id`.

**Error taxonomy:**

| Code | HTTP | Condition |
|---|---|---|
| `invalid_init_data` | 401 | HMAC mismatch |
| `expired_init_data` | 401 | `auth_date` outside window |
| `tenant_mismatch` | 403 | JWT tenant ≠ request tenant |
| `already_linked` | 409 | Account already has a Telegram identity |
| `telegram_id_already_used` | 409 | `from.id` linked to another account globally |

---

### 6.2 `POST /auth/telegram/unlink`

**Purpose:** Remove Telegram identity from an account.  
**Auth required:** Yes — valid Mycelium JWT.

**Validation steps:**

1. Verify JWT → `account_id`, `tenant_id`.
2. Assert `request.tenant_id == session.tenant_id`. If mismatch → 403.
3. Check account has `telegram_user` meta key. If absent → 404.
4. Remove `telegram_user` from `account.meta`.
5. Write audit record: `event=unlinked`.
6. Return 204.

---

### 6.3 `POST /auth/telegram/login`

**Purpose:** Authenticate via Telegram Mini App initData, produce a Mycelium session.  
**Auth required:** No — this *is* the authentication.

**Request:**
```json
{
  "tenant_id": "uuid",
  "init_data": "<raw initData>"
}
```

**Validation steps:**

1. Load `BotToken` for `tenant_id` via `TelegramConfigPort`.
2. Call `verify_init_data(...)`. If error → 401.
3. Extract `TelegramUserId`.
4. Reverse-lookup: query `account.meta` for `{ telegram_user: { id: <TelegramUserId> } }` using the GIN index (global — personal accounts have no `tenant_id`).
5. If not found → 401 (`telegram_id_not_linked`). Do not reveal whether the tenant exists.
6. Load the account and build a `Profile`.
7. Issue a `UserAccountScope` connection string (HMAC-signed, Base64-encoded) via the existing `UserAccountScope::new(...)` constructor in `core/src/domain/dtos/token/connection_string/user_account_connection_string.rs`. Expiry: configurable via `AccountLifeCycle` config, defaulting to the standard token TTL.
8. Write audit record: `event=login_ok`.
9. Return connection string.

**Response body:**
```json
{
  "connection_string": "<Base64-encoded UserAccountScope>",
  "expires_at": "2026-04-19T14:00:00Z"
}
```

**Usage after login:** the caller includes the connection string in subsequent requests to any Mycelium MCP or REST endpoint. Mycelium accepts both Bearer token (JWT) and connection string as authentication — the connection string issued here works through the existing connection-string auth path without any new infrastructure.

**On any auth failure:** write `event=login_fail` (with `telegram_id` if extracted, omitted otherwise).

**Rate limit:** 20 requests / 5 minutes per (`tenant_id` + `from.id`). Enforced before DB lookup.

---

### 6.4 `POST /webhooks/telegram/{tenant_id}`

**Purpose:** Receive Telegram updates (messages) directly from the Telegram Bot API.  
**Auth required:** `X-Telegram-Bot-Api-Secret-Token` header verification.

**Request constraints:**
- Body size cap: 512 KB. Return 413 if exceeded (before any parsing).
- Content-Type must be `application/json`. Return 415 if not.

**Validation steps (in order):**

1. Check body size ≤ 512 KB. If exceeded → 413.
2. Extract `X-Telegram-Bot-Api-Secret-Token` header.
3. Load `WebhookSecret` for `tenant_id` via `TelegramConfigPort`. If tenant not found or not configured → silently return 200 (do not reveal). Log `WARN` internally.
4. Call `verify_webhook_secret(header, webhook_secret)`. If mismatch → silently return 200. Log `WARN` with `tenant_id`.
5. Parse body as `TelegramUpdate` (typed struct, not raw JSON). If parse error → return 200 (Telegram will retry; log `ERROR`).
6. Extract `update_id` (`TelegramUpdateId`). Check KV for key `telegram:dedup:{tenant_id}:{update_id}`. If exists → return 200 (idempotent, do not forward).
7. Store `telegram:dedup:{tenant_id}:{update_id}` in KV with TTL = 86400 s (24 h).
8. Extract `from.id` from `message.from` or `callback_query.from`. If not present → return 200 (bot messages, channel posts, etc. are silently ignored).
9. Reverse-lookup personal account by `from.id` using GIN index (global — no `tenant_id` filter).
10. If not found → return 200. Log `INFO` (`telegram_id_not_linked`). Do not forward.
11. Build `Profile` for the resolved account.
12. Inject profile headers and route to the tenant's configured n8n downstream via the existing gateway pipeline (callback engines / `inject_downstream_secret`).
13. Return 200 to Telegram regardless of downstream outcome. Log downstream errors at `WARN`.

**Telegram requirement:** Must respond 200 within 5 seconds. Downstream forwarding is fire-and-forget (spawn or use existing async callback engine).

**Rate limit:** 100 requests / 10 seconds per `tenant_id`. Enforced at step 1 before secret resolution.

---

## 7. Route DTO extension (`identity_source`)

Add to `core/src/domain/dtos/route.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum IdentitySource {
    Telegram,
    // WhatsApp added in separate spec
}

// In Route struct:
#[serde(skip_serializing_if = "Option::is_none")]
pub identity_source: Option<IdentitySource>,
```

When `identity_source: Some(Telegram)` is set on a route, `check_security_group` adds a new branch **before** the existing `security_group` match:

```
if route.identity_source == Some(IdentitySource::Telegram):
    1. Verify source reliability (n8n IP must be in allowlist)
    2. Parse body JSON — extract `from.id`
    3. Reverse-lookup account by `from.id` + `tenant_id` (from route context or header)
    4. Build Profile
    5. Inject profile headers
    6. Continue pipeline as if SecurityGroup::Protected resolved normally
```

**Security invariant:** Source reliability check is mandatory and non-bypassable when `identity_source` is set. Without it, any caller could forge a `from.id`.

---

## 8. Architecture placement

| Component | Layer | File |
|---|---|---|
| `TelegramUser`, `TelegramUserId`, `BotToken`, `WebhookSecret`, `InitData`, `AuthDate`, `TelegramUpdateId` | `lib/http_tools/src/telegram/types.rs` | New |
| `verify_init_data` | `lib/http_tools/src/telegram/verify_init_data.rs` | New |
| `verify_webhook_secret` | `lib/http_tools/src/telegram/verify_webhook_secret.rs` | New |
| `TelegramConfigPort` trait | `core/src/domain/entities/telegram_config.rs` | New |
| `link_telegram_identity` use-case | `core/src/use_cases/gateway/telegram/link.rs` | New |
| `unlink_telegram_identity` use-case | `core/src/use_cases/gateway/telegram/unlink.rs` | New |
| `login_via_telegram` use-case | `core/src/use_cases/gateway/telegram/login.rs` | New |
| `resolve_account_by_telegram_id` use-case | `core/src/use_cases/gateway/telegram/resolve.rs` | New |
| `TelegramConfigAdapter` (Vault resolver) | `adapters/service/src/telegram_config.rs` | New |
| `POST /auth/telegram/link` handler | `ports/api/src/rest/gateway/telegram/link.rs` | New |
| `POST /auth/telegram/unlink` handler | `ports/api/src/rest/gateway/telegram/unlink.rs` | New |
| `POST /auth/telegram/login` handler | `ports/api/src/rest/gateway/telegram/login.rs` | New |
| `POST /webhooks/telegram/{tenant_id}` handler | `ports/api/src/rest/gateway/telegram/webhook.rs` | New |
| `TelegramMetaKey` variants | `core/src/domain/dtos/tenant/meta.rs` | Extend |
| `IdentitySource` enum + field on `Route` | `core/src/domain/dtos/route.rs` | Extend |
| `check_security_group` branch | `ports/api/src/router/check_security_group.rs` | Extend |
| GIN index migration | `adapters/diesel/sql/XXXX_telegram_gin_index.sql` | New |
| Unique index migration | `adapters/diesel/sql/XXXX_telegram_unique_link.sql` | New |
| Audit table migration | `adapters/diesel/sql/XXXX_telegram_identity_audit.sql` | New |

**Dependency constraints (hexagonal):**
- `verify_init_data` and `verify_webhook_secret` are pure functions in `lib/http_tools`. No `core` dependency.
- `TelegramConfigPort` is a trait in `core/domain/entities/`. `core` does not import any adapter.
- `link_telegram_identity` and other use-cases receive `&dyn TelegramConfigPort` and `&dyn AccountMetaRepo` as trait objects. No concrete adapter imported.
- Handlers in `ports/api` wire concrete adapters via `shaku` DI, as done elsewhere.

---

## 9. Open questions

### OQ-1 — What does `/auth/telegram/login` return? ✅ RESOLVED

**Decision:** Returns a `UserAccountScope` connection string — the same HMAC-signed, Base64-encoded token already used throughout the system. Issued via `UserAccountScope::new(...)` in `core/src/domain/dtos/token/connection_string/`. No new token infrastructure.

**Rationale:** Two authentication modes coexist (see §1b). Mode A (agents/MCP) uses this connection string for subsequent calls. Mode B (n8n Leg 2) uses body passthrough with IP allowlist. Both are supported simultaneously.

### OQ-2 — Who manages `setWebhook` registration? ✅ RESOLVED

**Decision:** Manual. The tenant admin calls the Telegram API directly (`api.telegram.org/setWebhook`) outside of Mycelium. Mycelium only stores the bot token and webhook secret in Vault and acts as a passive receiver. No `setWebhook` automation, no outbound HTTP to Telegram from Mycelium.

### OQ-2b — Can the same Telegram user be linked to multiple tenants? ✅ SUPERSEDED

**Original decision (2026-04-19):** Per-tenant uniqueness — same `from.id` could link to different accounts in different tenants.

**Revised decision (2026-04-19):** Global uniqueness. Telegram identity links to a **personal account** (user/manager/staff), not a subscription account. Personal accounts have no `tenant_id` column, so per-tenant deduplication is impossible and semantically wrong.

A `TelegramUserId` maps to exactly **one personal account globally**. Multi-tenant access is handled by the account's guest memberships — the same person can call `POST /auth/telegram/login/{tenant_id}` for any tenant they belong to, receiving a connection string scoped to that tenant.

**Implication for the unique index:** global constraint on `from.id` only — see revised §5.2.

**Implication for reverse-lookup:** `get_by_telegram_id` takes no `tenant_id`. The `tenant_id` is used only when issuing the scoped connection string after the account is found.

### OQ-3 — Bot token rotation procedure? ✅ RESOLVED

**Decision:** Vault-only, manual update. No cache — the bot token is fetched from Vault on every request via `TelegramConfigPort`. Rotation is instant: the admin updates the Vault value and the next request picks it up automatically. No invalidation endpoint needed.

---

## 10. New crate dependencies

| Crate | Where | Reason |
|---|---|---|
| `subtle` or `constant_time_eq` | `lib/http_tools` | Constant-time HMAC comparison |
| `secrecy` | `lib/http_tools`, `core` | `SecretString` for bot token / webhook secret |
| `hmac` + `sha2` | `lib/http_tools` | HMAC-SHA256 for initData verification |
| `zeroize` | `lib/http_tools` | Derived by `secrecy`; explicit for newtype fields |

`hmac` and `sha2` are likely already present (check `Cargo.toml` before adding). Prefer extending existing deps over adding new ones per the security rules.
