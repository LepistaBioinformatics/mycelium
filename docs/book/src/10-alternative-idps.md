# Alternative Identity Providers

Mycelium supports identity providers beyond the built-in email/password and magic-link flows.
Alternative IdPs allow users to authenticate using credentials from third-party platforms
(e.g. Telegram) and downstream services to receive the same `x-mycelium-profile` injection
they get from any other authentication method.

---

## Concepts

### Identity linking vs. authentication

An alternative IdP works in two stages:

1. **Linking** — A user with an existing Mycelium account authorizes the IdP to act on their behalf.
   The link is stored on the account (personal account, not tenant-scoped).

2. **Authentication** — The IdP credential is presented to Mycelium in exchange for a connection string
   (JWT) or, in the webhook case, the credential is extracted from an inbound request body.

A user who has not linked their IdP identity cannot authenticate via that IdP.

### Personal vs. subscription accounts

IdP links are stored on **personal accounts** (accounts with no `tenant_id`). Personal accounts are
cross-tenant: a user who belongs to multiple tenants links their Telegram once, and the link is valid
for all tenants they are a member of.

Subscription accounts (tenant-scoped) cannot hold IdP links.

### Per-tenant configuration

Some IdPs (Telegram) require per-tenant credentials (bot token, webhook secret). These are stored
encrypted in the tenant metadata. The tenant owner is responsible for provisioning them.

This configuration is **required only for operations that verify Telegram credentials**:
linking a new identity and logging in. It is **not required** for body-based identity
resolution in webhook routes — that lookup is global (see
[What works without tenant config](#what-works-without-tenant-config) below).

---

## Telegram

### Prerequisites

- A Telegram bot created via [@BotFather](https://t.me/BotFather). You will receive a **bot token**.
- A **webhook secret** — any random string of your choice (16–256 characters). You choose it; you
  will supply it both to Mycelium and to Telegram's `setWebhook` call.
- The Mycelium gateway must be reachable from the internet (or from Telegram's servers) for the
  webhook use case.

---

### Admin Journey: Provisioning the Tenant

The tenant owner performs this once per tenant, before any user can link or log in.

#### Step 1 — Store bot credentials in Mycelium

```http
POST /_adm/tenant-owner/telegram/config
Authorization: Bearer <tenant-owner-connection-string>
x-mycelium-tenant-id: <tenant-uuid>
Content-Type: application/json

{
  "botToken": "<BotFather token>",
  "webhookSecret": "<your chosen webhook secret>"
}
```

- Response: `204 No Content` on success.
- Both values are encrypted with AES-256-GCM before storage. They are never returned in plaintext.
- This endpoint is gated to accounts with the **tenant-owner** role on the given tenant.

#### Step 2 — Register the webhook with Telegram (webhook use case only)

Skip this step if you only need the login flow (Use Case B below).

```bash
curl -X POST "https://api.telegram.org/bot<token>/setWebhook" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://<your-gateway-host>/auth/telegram/webhook/<tenant-uuid>",
    "secret_token": "<your chosen webhook secret>"
  }'
```

Telegram will send all bot updates to that URL and sign them with
`X-Telegram-Bot-Api-Secret-Token: <webhook secret>`. Mycelium verifies this header before
accepting any update.

---

### What works without tenant config

The table below shows exactly which operations require the tenant to have Telegram configured
(admin Step 1 completed) and which do not.

| Operation | Requires tenant config? | Why |
|---|---|---|
| Link Telegram identity (`POST /auth/telegram/link`) | **Yes** | Verifies `initData` HMAC against the tenant's bot token |
| Login via Telegram (`POST /auth/telegram/login/{tenant_id}`) | **Yes** | Same — HMAC verification against tenant bot token |
| Webhook route profile resolution (`identitySource = "telegram"`) | **No** | Global lookup by Telegram user ID; no tenant credential involved |

**Consequence for multi-tenant setups:**

A user who linked their Telegram identity in *any* tenant that has Telegram configured holds
a global link (stored on their personal account). That link is valid across all tenants they
belong to — including tenants with no Telegram configuration at all.

```
Tenant A  (Telegram configured)      Tenant B  (no Telegram config)
   │                                      │
   │  User links via Tenant A's bot       │  Webhook route uses identitySource
   │  → link stored on personal account  │  → gateway resolves profile globally
   │                                      │  → x-mycelium-profile injected ✓
   │  User cannot link via Tenant B ✗    │
   │  User cannot login via Tenant B ✗   │
```

In other words: **link once via any configured tenant; the identity is then available
everywhere for webhook resolution.** The login flow is always tenant-specific because it
issues a connection string scoped to a particular tenant.

---

### User Journey: Linking a Telegram Identity

Each user must link their Telegram account before they can authenticate via it.
Linking requires going through a tenant that has Telegram configured (admin Step 1).
This is done through a **Telegram Mini App** that runs inside that tenant's bot chat.

The Mini App obtains `initData` from the Telegram Web App context
(`window.Telegram.WebApp.initData`) and sends it to the link endpoint:

```http
POST /_adm/auth/telegram/link
Authorization: Bearer <user-connection-string>
x-mycelium-tenant-id: <tenant-uuid>
Content-Type: application/json

{
  "initData": "<Telegram Mini App initData string>"
}
```

- Response: `204 No Content` on success.
- The `initData` HMAC is verified against the bot token configured for the supplied tenant.
- The Telegram user ID is written to the authenticated Mycelium **personal account** metadata.
  This link is global — valid for all tenants the user belongs to.
- Returns `409` if the account already has a Telegram link, or if the Telegram ID is already
  linked to another account globally.
- Returns `422` if the tenant has not completed admin Step 1.

To remove a link:

```http
DELETE /_adm/auth/telegram/link
Authorization: Bearer <user-connection-string>
```

---

### Use Case A — Login + API Calls (Mini App → downstream APIs)

**When to use:** An AI agent or a Telegram Mini App needs to call downstream APIs that are
protected by Mycelium. Telegram serves as the identity provider; the resulting connection string
is used as a Bearer token for regular API calls.

#### Flow

```
Mini App (in Telegram)
  │
  │  1. Obtain initData from Telegram.WebApp.initData
  │
  ▼
POST /auth/telegram/login/{tenant_id}      (public, no Bearer required)
  │
  │  2. Mycelium verifies initData HMAC against stored bot token
  │  3. Resolves linked Mycelium account by Telegram user ID
  │  4. Issues a connection string scoped to tenant_id
  │
  ▼
{ "connectionString": "...", "expiresAt": "..." }
  │
  │  5. Mini App uses connection_string as Bearer token
  │
  ▼
GET /my-service/some-resource              (authenticated or protected route)
Authorization: Bearer <connection_string>
  │
  ▼
Downstream service receives x-mycelium-profile header
```

#### Login endpoint

```http
POST /_adm/auth/telegram/login/{tenant_id}
Content-Type: application/json

{
  "initData": "<Telegram Mini App initData string>"
}
```

Response:

```json
{
  "connectionString": "<scoped JWT>",
  "expiresAt": "2026-04-20T18:00:00-03:00"
}
```

- Public endpoint — no `Authorization` header required.
- Returns `401` if initData is invalid or expired.
- Returns `404` if the Telegram user has not linked their account in this tenant.
- Returns `422` if the tenant has not completed admin Step 1.

#### Gateway configuration

No special gateway config is needed. The downstream service uses the same `authenticated` or
`protected` groups as any other authenticated user.

```toml
[[my-bot-api]]
host = "my-api:3000"
protocol = "http"

[[my-bot-api.path]]
group = "protected"
path = "/api/*"
methods = ["ALL"]
```

---

### Use Case B — Webhook Routes (Telegram bot → downstream service, authenticated)

**When to use:** Your downstream service exposes a Telegram webhook handler and needs to know
*which Mycelium user* sent a given message. Telegram sends updates to Mycelium; Mycelium
resolves the sender's identity and injects `x-mycelium-profile` before forwarding to your
service.

#### Flow

```
Telegram servers
  │
  │  1. User sends message to bot
  │  2. Telegram POSTs Update to gateway webhook endpoint
  │     with X-Telegram-Bot-Api-Secret-Token header
  │
  ▼
POST /_adm/auth/telegram/webhook/{tenant_id}    (gateway internal endpoint)
  │
  │  3. Mycelium verifies webhook secret
  │  4. Returns 200 to Telegram immediately
  │
  ▼
  (Route with identitySource = "telegram")
  │
  │  5. Gateway buffers the Update body
  │  6. Extracts from.id from the Telegram Update object
  │  7. Resolves linked Mycelium account by Telegram user ID
  │  8. Injects x-mycelium-profile into the forwarded request
  │
  ▼
Downstream service receives the Update body + x-mycelium-profile
```

#### Gateway configuration

Two fields enable this on the gateway config:

- `allowedSources` on the service — restricts which `Host` headers are accepted (source
  reliability check, evaluated before body parsing)
- `identitySource` on the route — tells the gateway to extract identity from the body

```toml
[[my-bot-handler]]
host = "my-bot-service:3000"
protocol = "http"
allowedSources = ["api.telegram.org"]          # mandatory when identitySource is set

[[my-bot-handler.path]]
group = "protected"
path = "/telegram/webhook"
methods = ["POST"]
identitySource = "telegram"
```

`allowedSources` accepts wildcards (`"*.telegram.org"`, `"10.0.0.*"`). If it is missing and
`identitySource` is set, the gateway rejects the route configuration at request time.

#### Important constraints

- The user must have linked their Telegram account (User Journey above) before the gateway
  can resolve their profile. Updates from unlinked users return `401` to the downstream.
- The `group` can be `protected` or `protectedByRoles` — the gateway will enforce the declared
  security group after injecting the profile.
- Your downstream service receives the **full Telegram Update JSON** in the body, unchanged.
  The profile is injected only as the `x-mycelium-profile` header.

---

### Comparison: Use Case A vs. Use Case B

| | Use Case A (Login + API calls) | Use Case B (Webhook routes) |
|---|---|---|
| **Who calls Mycelium** | Your Mini App / AI agent | Telegram's servers |
| **Authentication mechanism** | `initData` exchanged for a JWT | Webhook secret + body identity extraction |
| **Tenant Telegram config required** | Yes — for every tenant used for login | No — identity lookup is global |
| **Gateway config required** | Standard `authenticated`/`protected` routes | `allowedSources` + `identitySource = "telegram"` |
| **Webhook registration with Telegram** | Not required | Required (`setWebhook`) |
| **User must have linked identity** | Yes — via any configured tenant | Yes — via any configured tenant |
| **Connection string issued** | Yes — reused for multiple calls | No — each request is re-verified |
| **Typical use** | Mini App calling your API | Bot receiving user messages in a downstream handler |

---

### Troubleshooting

**`422 telegram_not_configured_for_tenant`**
: Applies to linking and login only. Admin has not called `POST /tenant-owner/telegram/config`
  for this tenant. Complete admin Step 1. Note: webhook route identity resolution does **not**
  require tenant config — if you are seeing this on a webhook route, the request reached the
  wrong endpoint.

**User is a guest in a tenant with no Telegram config — what still works?**
: Webhook routes with `identitySource = "telegram"` still work if the user previously linked
  their Telegram identity via any other tenant. Login and re-linking via the unconfigured tenant
  are not possible until the tenant owner completes admin Step 1.

**`401 invalid_telegram_init_data`**
: The `initData` HMAC check failed. Causes: wrong bot token in config, expired initData
  (Telegram initData is valid for a short window after it is issued), or tampered data.

**`404` on login**
: The Telegram user ID is not linked to any Mycelium account in this tenant.
  The user must complete the linking journey first.

**`401` on webhook route (profile resolution failed)**
: The sender has not linked their Telegram identity. The gateway logs
  `"Telegram identity not linked to any account"`.

**Webhook `401 invalid_webhook_secret`**
: The `X-Telegram-Bot-Api-Secret-Token` header does not match the stored secret.
  Verify that the secret in Mycelium (`/tenant-owner/telegram/config`) matches the
  `secret_token` you supplied in `setWebhook`.

**`allowedSources` not working as expected**
: Remember that `allowedSources` checks the `Host` header, not `Origin`. Server-to-server
  calls (Telegram → your gateway) send `Host: api.telegram.org`, not an `Origin` header.
  CORS (`allowedOrigins`) is irrelevant for webhook routes. See
  [Downstream APIs](./06-downstream-apis.md#body-based-identity-providers-webhook-routes).
