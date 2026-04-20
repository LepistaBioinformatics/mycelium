# Alternative Identity Providers

By default, Mycelium authenticates users through email + magic link. Alternative IdPs let users
prove who they are using an account they already have on another platform — for example, Telegram.

When authentication succeeds, the downstream service receives the same `x-mycelium-profile`
header it would receive from any other authentication method. From the downstream's perspective,
the identity source is irrelevant: a user is a user.

---

## How it works — the big picture

Before a user can authenticate via an alternative IdP, two things must be true:

1. **The tenant admin has configured the IdP** — each IdP requires credentials specific to that
   tenant (e.g. a Telegram bot token). Without this, authentication is not possible.

2. **The user has linked their IdP identity to their Mycelium account** — this is a one-time step
   where the user says "my Telegram account is me". After linking, the user can authenticate
   via Telegram any time, in any tenant they belong to.

Think of it like adding a phone number to a bank account: the bank (Mycelium) holds your regular
credentials, but you can also prove your identity by receiving an SMS to your registered number
(Telegram). You register the number once; after that, it just works.

---

## Concepts

### Identity linking vs. authentication

An alternative IdP works in two stages:

1. **Linking** — The user connects their IdP identity to their Mycelium account. This is done
   once. The link is stored on the personal account and is global across tenants.

2. **Authentication** — The user presents their IdP credential. Mycelium verifies it and either
   issues a connection string (JWT) or, for webhook routes, resolves the profile directly from
   the incoming request body.

A user who has not linked their IdP identity cannot authenticate via that IdP.

### Personal vs. subscription accounts

IdP links are stored on **personal accounts**. A personal account belongs to a person, not to
a specific tenant. When a user belongs to multiple tenants (e.g. a contractor who works for
two companies on the same Mycelium installation), they link their Telegram once and it works
for all tenants they are a member of.

Subscription accounts are tenant-scoped and cannot hold IdP links.

### Per-tenant configuration

Some IdPs (Telegram) require credentials that are specific to a tenant — for example, a Telegram
bot token. A bot is created by the company, not by the user. Each company (tenant) has its own
bot, so each tenant stores its own credentials.

This configuration is **required only for operations that verify Telegram credentials**:
linking a new identity and issuing login tokens. It is **not required** for body-based identity
resolution in webhook routes — that lookup is global (see
[What works without tenant config](#what-works-without-tenant-config) below).

---

## Telegram

### Prerequisites

- A Telegram bot created via [@BotFather](https://t.me/BotFather). After creating the bot you
  receive a **bot token** (looks like `7123456789:AAF...`). Keep it secret.
- A **webhook secret** — a random string you choose (16–256 characters). This is not a Telegram
  credential; it is a shared secret between Mycelium and Telegram so that Mycelium can verify
  that incoming webhook calls really come from Telegram and not from an attacker.
- The Mycelium gateway must be reachable from the internet for the webhook use case. For the
  login-only use case, it only needs to be reachable from the users' devices.

---

### Admin Journey: Provisioning the Tenant

**Who does this:** The tenant owner — the person or team responsible for the company's Mycelium
tenant. This is done **once**, before any user can link or log in.

**Concrete example:** Acme Corp runs a Mycelium installation. Their IT admin, Carlos, manages the
tenant. Carlos created a Telegram bot called `@AcmeHRBot` via BotFather and received the bot token.
He also generated a random webhook secret (`openssl rand -hex 32`). Now he needs to store these in
Mycelium so the gateway can use them.

#### Step 1 — Store bot credentials in Mycelium

Carlos calls this endpoint using his tenant-owner connection string:

```http
POST /_adm/tenant-owner/telegram/config
Authorization: Bearer <carlos-connection-string>
x-mycelium-tenant-id: a3f1e2d0-1234-4abc-8def-000000000001
Content-Type: application/json

{
  "botToken": "7123456789:AAFexampleBotTokenFromBotFather",
  "webhookSecret": "4b9c2e1a8f3d7e0c5b2a9f6e3d1c8b5a"
}
```

- Response: `204 No Content`.
- Both values are encrypted with AES-256-GCM before storage. They are never readable again
  through the API — if lost, they must be re-submitted.
- Only accounts with the **tenant-owner** role can call this endpoint.

After this step, users can link their Telegram to their Mycelium accounts and log in.

#### Step 2 — Register the webhook with Telegram (webhook use case only)

> Skip this step if you only need the login flow (Use Case A below).

Carlos tells Telegram where to send bot updates. He uses the Telegram Bot API directly:

```bash
curl -X POST "https://api.telegram.org/bot7123456789:AAFexampleBotTokenFromBotFather/setWebhook" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://gateway.acme.com/auth/telegram/webhook/a3f1e2d0-1234-4abc-8def-000000000001",
    "secret_token": "4b9c2e1a8f3d7e0c5b2a9f6e3d1c8b5a"
  }'
```

From this point on, every time someone sends a message to `@AcmeHRBot`, Telegram will POST the
update to that URL and include the header `X-Telegram-Bot-Api-Secret-Token: 4b9c2e1...`. Mycelium
verifies that header before doing anything with the update.

---

### User Journey: Linking a Telegram Identity

**Who does this:** Each end user, once, using a Telegram Mini App built by the company.

**Concrete example:** Maria is an Acme employee. She has a Mycelium account (registered via magic
link to `maria@acme.com`) and belongs to Acme's tenant. Now she wants to use `@AcmeHRBot` to check
her vacation balance. Before she can do anything with the bot, she needs to link her Telegram
account to her Mycelium account.

Carlos built a Telegram Mini App (a small web page that opens inside Telegram). When Maria opens
it, the app reads the cryptographically signed `initData` that Telegram injects into every Mini App
session. This `initData` proves to Mycelium that the Telegram user opening the app is who they claim
to be, because it is signed with the bot token.

The Mini App sends:

```http
POST /_adm/auth/telegram/link
Authorization: Bearer <maria-connection-string>
x-mycelium-tenant-id: a3f1e2d0-1234-4abc-8def-000000000001
Content-Type: application/json

{
  "initData": "query_id=AAH...&user=%7B%22id%22%3A98765432%2C%22username%22%3A%22maria_acme%22...&hash=abc123..."
}
```

What Mycelium does:
1. Verifies the HMAC in `initData` using Acme's bot token — confirms this is a real Telegram user.
2. Extracts Maria's Telegram user ID (`98765432`) from `initData`.
3. Stores `{ id: 98765432, username: "maria_acme" }` in Maria's personal account metadata.

Maria only does this once. The link is stored on her personal account and is valid across all
tenants she belongs to.

- Returns `409` if Maria already has a Telegram link, or if Telegram ID `98765432` is already
  linked to another Mycelium account.
- Returns `422` if Carlos hasn't completed admin Step 1.

To remove the link later:

```http
DELETE /_adm/auth/telegram/link
Authorization: Bearer <maria-connection-string>
```

---

### Use Case A — Employee Mini App Calling Protected APIs

**The problem:** Acme built a web app inside Telegram (`@AcmeHRBot`) where employees can check
their remaining vacation days, submit expense reports, and view assigned tasks. The backend API
that serves this data is protected by Mycelium: it only responds to authenticated users and injects
their profile into every request. The Mini App cannot ask Maria to type her email and password —
she's already inside Telegram. Telegram is the identity.

**The solution:** The Mini App exchanges Telegram's `initData` for a Mycelium connection string.
That connection string is used as a Bearer token for all subsequent API calls. The backend API sees
a normal authenticated request and never knows the user logged in via Telegram.

#### Full journey

```
Maria opens the Mini App inside @AcmeHRBot on her phone
  │
  │  Telegram injects window.Telegram.WebApp.initData into the Mini App
  │  (this string is signed by Telegram and expires in ~24 hours)
  │
  ▼
Mini App calls the login endpoint (no credentials needed — it's public):

  POST /_adm/auth/telegram/login/a3f1e2d0-1234-4abc-8def-000000000001
  { "initData": "query_id=AAH...&user=...&hash=abc123..." }
  │
  │  Mycelium:
  │    1. Verifies the HMAC using Acme's bot token → confirms Maria is who she claims
  │    2. Extracts Telegram user ID 98765432
  │    3. Looks up which Mycelium account has this Telegram ID → finds maria@acme.com
  │    4. Issues a connection string scoped to Acme's tenant
  │
  ▼
  { "connectionString": "acc=...;tid=...;sig=...", "expiresAt": "2026-04-21T10:00:00-03:00" }
  │
  │  Mini App stores the connection string in memory for this session
  │
  ▼
Mini App calls the HR API:

  GET /hr-api/vacation-balance
  Authorization: Bearer acc=...;tid=...;sig=...
  │
  │  Mycelium gateway:
  │    - Validates the connection string
  │    - Resolves Maria's full profile (account, tenant membership, roles)
  │    - Injects x-mycelium-profile into the forwarded request
  │
  ▼
HR API service receives:
  GET /vacation-balance
  x-mycelium-profile: <base64 compressed JSON with Maria's identity, roles, tenant>
  │
  │  HR API reads the profile, finds Maria's account ID, returns her vacation balance
  │
  ▼
Mini App displays: "You have 12 vacation days remaining."
```

#### Login endpoint reference

```http
POST /_adm/auth/telegram/login/{tenant_id}
Content-Type: application/json

{
  "initData": "<Telegram Mini App initData string>"
}
```

Response on success:

```json
{
  "connectionString": "acc=uuid;tid=uuid;r=user;edt=2026-04-21T10:00:00-03:00;sig=...",
  "expiresAt": "2026-04-21T10:00:00-03:00"
}
```

- Public endpoint — no `Authorization` header required.
- Returns `401` if `initData` is invalid or expired.
- Returns `404` if Telegram user ID is not linked to any account in this tenant.
- Returns `422` if the tenant has not completed admin Step 1.

#### Gateway configuration

No special gateway config is needed for this use case. The HR API uses the same groups as any
other service:

```toml
[[hr-api]]
host = "hr-service:3000"
protocol = "http"

[[hr-api.path]]
group = "protected"            # requires a valid profile
path = "/hr-api/*"
methods = ["ALL"]
```

---

### Use Case B — Customer Support Bot with Authenticated Messages

**The problem:** Acme runs a customer support bot (`@AcmeSupportBot`). When a customer writes
"my order hasn't arrived", the support handler needs to know *who is writing* — their account,
subscription tier, and open tickets. Without identity, the bot can only reply generically.
With identity, it can reply "Hi Maria, your order #4521 shipped yesterday and arrives tomorrow."

The support handler is a downstream service behind Mycelium. It cannot issue JWTs or do its own
authentication — it just wants to receive the message *and* know who sent it. Mycelium handles
the identity resolution transparently.

**The solution:** Configure a gateway route with `identitySource = "telegram"`. When Telegram
sends an update, Mycelium extracts the sender's Telegram ID from the message body, looks up
their linked Mycelium account, and injects their profile before forwarding the update to the
support handler.

The support handler never sees unauthenticated messages on this route. If the sender hasn't
linked their Telegram account, Mycelium returns `401` and the message is not forwarded.

#### Full journey

```
Customer (Maria) sends "my order hasn't arrived" to @AcmeSupportBot
  │
  │  Telegram servers POST the update to Mycelium:
  │
  ▼
POST /_adm/auth/telegram/webhook/a3f1e2d0-1234-4abc-8def-000000000001
X-Telegram-Bot-Api-Secret-Token: 4b9c2e1a8f3d7e0c5b2a9f6e3d1c8b5a
{
  "update_id": 100000001,
  "message": {
    "from": { "id": 98765432, "username": "maria_acme" },
    "text": "my order hasn't arrived"
  }
}
  │
  │  Mycelium verifies the webhook secret → confirms this really came from Telegram
  │  Responds 200 OK immediately (Telegram requires this, or it will retry)
  │
  ▼
Gateway route (identitySource = "telegram") takes over:
  │
  │  1. Buffers the request body
  │  2. Extracts from.id = 98765432
  │  3. Looks up which Mycelium account has Telegram ID 98765432 → maria@acme.com
  │  4. Loads Maria's full profile (account, tenant, roles)
  │  5. Injects x-mycelium-profile into the forwarded request
  │
  ▼
Support handler receives:
  POST /telegram/webhook
  x-mycelium-profile: <base64 compressed JSON>
  {
    "update_id": 100000001,
    "message": { "from": { "id": 98765432, ... }, "text": "my order hasn't arrived" }
  }
  │
  │  Support handler reads the profile → finds Maria's account → fetches her orders
  │  Replies via Telegram Bot API: "Hi Maria, your order #4521 shipped yesterday."
```

#### Gateway configuration

```toml
[[acme-support-bot]]
host = "support-handler:3000"
protocol = "http"
allowedSources = ["api.telegram.org"]    # only accept requests from Telegram's servers

[[acme-support-bot.path]]
group = "protected"
path = "/telegram/webhook"
methods = ["POST"]
identitySource = "telegram"             # resolve identity from the message body
```

Two fields are required:

- **`allowedSources`** — before even parsing the body, Mycelium checks the `Host` header of the
  incoming request. Only requests whose `Host` matches this list are processed. This prevents
  an attacker from POSTing fake updates directly to your service.
  Supports wildcards: `"*.telegram.org"`, `"10.0.0.*"`.

- **`identitySource = "telegram"`** — tells Mycelium to extract the sender's identity from the
  body (via `message.from.id` or the equivalent field in other update types) instead of looking
  for a Bearer token.

If `allowedSources` is missing and `identitySource` is set, the gateway rejects the request.

#### Important constraints

- **The user must have previously linked their Telegram account.** Messages from users who
  haven't linked return `401`. Consider having the bot reply with a link to the Mini App
  where the user can complete the linking step.
- The `group` field still applies. Use `protected` if all you need is the user's profile. Use
  `protectedByRoles` if you want to further restrict which users can interact with the bot.
- Your support handler receives the **full Telegram Update JSON** unchanged in the request body.
  The profile is injected as a header, not embedded in the body.

---

### What works without tenant config

**The problem:** Acme has two tenants — `acme-hr` and `acme-operations`. The HR tenant has
Telegram configured; the operations tenant does not. Maria belongs to both tenants and has
already linked her Telegram via the HR tenant's Mini App.

What can Maria do in the operations tenant?

| Operation | Requires tenant config? | Maria in acme-operations |
|---|---|---|
| Link Telegram identity | **Yes** — verifies `initData` HMAC against the tenant's bot token | Cannot link via this tenant |
| Login via Telegram | **Yes** — same HMAC verification | Cannot log in via Telegram to this tenant |
| Appear in webhook route identity resolution | **No** — global lookup by Telegram user ID | Works — her link from acme-hr is found globally |

The key insight: **the link is stored on Maria's personal account, not on the tenant**. When
Mycelium receives a webhook update from operations' bot and finds `from.id = 98765432`, it looks
up globally — finds Maria's personal account — and injects her profile. The operations tenant
never needed to know about Telegram.

```
acme-hr  (Telegram configured)          acme-operations  (no Telegram config)
   │                                            │
   │  Maria links via HR Mini App              │  Operations has a support bot
   │  → Telegram ID 98765432                   │  → route: identitySource = "telegram"
   │    stored on Maria's personal account     │
   │                                            │  Telegram sends update with from.id 98765432
   │                                            │  → Mycelium finds Maria globally ✓
   │                                            │  → x-mycelium-profile injected ✓
   │                                            │
   │  Maria cannot link via operations ✗       │  Maria cannot login via Telegram here ✗
   │  Maria cannot login via Telegram here ✗   │  (no bot token to verify initData)
```

**In practice:** If you are building a webhook-only integration (Use Case B) for a tenant, you do
not need to configure Telegram for that tenant — as long as users have linked in some other tenant.
If you need login (Use Case A) for that tenant, the tenant must have its own bot and must complete
the admin provisioning step.

---

### Comparison: Use Case A vs. Use Case B

| | Use Case A — Login + API calls | Use Case B — Webhook identity resolution |
|---|---|---|
| **Example** | Mini App calling an HR API | Support bot knowing who sent a message |
| **Who calls Mycelium** | Your Mini App / AI agent | Telegram's servers |
| **Authentication mechanism** | `initData` → connection string (JWT) | Webhook secret + sender ID from body |
| **Tenant Telegram config required** | Yes — every tenant used for login | No — global identity lookup |
| **Gateway route config** | Standard `authenticated`/`protected` | `allowedSources` + `identitySource = "telegram"` |
| **Webhook registration with Telegram** | Not required | Required (`setWebhook`) |
| **User must have linked identity** | Yes — via any configured tenant | Yes — via any configured tenant |
| **Connection string issued** | Yes — reused for the session | No — identity re-resolved per request |
| **What the downstream receives** | `x-mycelium-profile` on any route | Full Telegram Update body + `x-mycelium-profile` |

---

### Troubleshooting

**`422 telegram_not_configured_for_tenant`**
: The tenant has not completed admin Step 1 (`POST /tenant-owner/telegram/config`). This error
  appears on link and login. It does **not** appear on webhook routes (identity resolution there
  is global and does not need tenant config).

**User is a guest in a tenant with no Telegram config — what still works?**
: Webhook routes with `identitySource = "telegram"` still work if the user previously linked
  their Telegram identity via any other tenant. Login and linking via the unconfigured tenant
  are not possible until the tenant owner completes admin Step 1.

**`401 invalid_telegram_init_data`**
: The `initData` HMAC check failed. Causes: wrong bot token stored in Mycelium, `initData`
  expired (Telegram signs `initData` with a short-lived timestamp), or the string was modified
  in transit. Re-read `initData` from `window.Telegram.WebApp.initData` and retry immediately.

**`404` on login**
: The Telegram user ID extracted from `initData` is not linked to any Mycelium account in this
  tenant. The user must complete the linking step first (open the Mini App that calls
  `POST /auth/telegram/link`).

**`401` on webhook route — message not forwarded**
: The sender has not linked their Telegram account. The gateway returns `401` and does not
  forward the update to your service. Handle this in your bot by detecting that the webhook
  call did not reach your service (or implement a fallback public route) and sending the user
  a message with a link to the linking Mini App.

**Webhook `401 invalid_webhook_secret`**
: The `X-Telegram-Bot-Api-Secret-Token` header sent by Telegram does not match the secret
  stored in Mycelium. Verify that the `webhookSecret` you supplied in Step 1 exactly matches
  the `secret_token` you passed to `setWebhook`. They must be identical strings.

**`allowedSources` not working as expected**
: `allowedSources` checks the `Host` header, not `Origin`. Telegram's servers send requests
  with `Host: api.telegram.org`. `Origin` is a browser header sent by CORS preflight requests
  — Telegram never sends it. CORS (`allowedOrigins`) is completely independent and irrelevant
  for webhook routes. See
  [Downstream APIs](./06-downstream-apis.md#body-based-identity-providers-webhook-routes).
