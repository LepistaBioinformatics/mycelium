# Messaging Platform IdP — Design Decisions

**Status:** Under discussion — roadmap entry not yet written  
**Date:** 2026-04-06  
**Next step:** Write the ROADMAP.md entry based on the decisions below

---

## What we want to build

WhatsApp and Telegram as Identity Providers in Mycelium. Users who already have a Mycelium
account can link their WA/Telegram identity to the account. Once linked, the platform identity
can be used to authenticate requests in the gateway.

---

## Discussion iterations

### Iteration 1 — Scope and data model

**Decision:** Identity linked to the **account**, not at the user-level.  
**Reason:** Personal accounts have a single primary owner. User-level linkage would add complexity
with no benefit for the intended use case.

**Decision:** `AccountMetaKey::WhatsAppUser` and `AccountMetaKey::TelegramUser` already exist in
`core/src/domain/dtos/account/meta.rs`. Reused as storage for the linked identity.

**Decision:** For authentication, a reverse lookup is required: given a `from.id` or `wa_id`,
find the account. This requires a GIN index on the `account.meta` (JSONB) field in Postgres.

---

### Iteration 2 — Verification flow on linking

**WhatsApp:** Has no Mini App or equivalent. Linking flow via WA Business API:
- Mycelium generates a short-lived code
- User sends the code to the tenant's WA Business number
- Mycelium receives it via webhook, validates `X-Hub-Signature-256`, links `wa_id` to the account

**Telegram:** Via Mini App.
- Mini App sends `initData` to `POST /auth/telegram/link`
- Mycelium verifies HMAC-SHA256 locally using the tenant's bot token (no round-trip)
- `from.id` and `from.username` linked to the account

**Security invariant:** `wa_id` and `from.id` are never self-declared. They are only stored
after a challenge verified by the platform.

---

### Iteration 3 — How the platform identity is used to authenticate

**Discarded:** Trusting n8n as an identity intermediary (n8n adds a header with `from.id`,
Mycelium trusts it). Problem: if n8n is compromised, any identity can be forged. No security
guarantee.

**Decision:** Mycelium receives webhooks **directly** from the platforms.

- Telegram: validates `X-Telegram-Bot-Api-Secret-Token`
- WhatsApp: validates `X-Hub-Signature-256` (HMAC-SHA256 of the raw body with the app secret)

Mycelium becomes the entry point for platform webhooks. N8n sits downstream,
not as an identity intermediary.

**Reason:** Trust comes from the platform (Telegram/Meta), not from n8n. A compromised n8n
cannot forge a valid signature without knowing the tenant secrets, which are stored in Mycelium.

---

### Iteration 4 — Gateway architecture for Telegram/WA

**Path A (chosen):** Mycelium as webhook receiver. Platform delivers directly to
Mycelium, which validates, resolves the account, and routes to n8n as a registered downstream.

**Path B (discarded):** n8n receives from the platform, passes original headers to Mycelium.
Problem for WA: `X-Hub-Signature-256` is an HMAC of the entire body — if n8n transforms the
body, the signature breaks.

**Reason for Path A:** Mycelium already has downstream callbacks (HTTP, Rhai, JS, Python). N8n
is a registered downstream in the gateway, not an identity intermediary.

---

### Iteration 5 — How n8n calls other downstream services with the user's identity

**Attempt 1 — Shared secret (HMAC between n8n and Mycelium):** Discarded. N8n has no native
HMAC signing mechanism for outgoing requests. Would require a Code node in every workflow.

**Attempt 2 — Static token per tenant:** Discarded. Does not guarantee n8n is secure;
a compromised token allows forging any identity.

**Attempt 3 — Short-lived token issued by Mycelium:** Discarded. Mycelium does not issue
short-lived tokens on the fly.

**Attempt 4 — Connection string generated at linking, stored in KV:**
Technically viable, but would add complexity to the linking flow and webhook ingestion.

**Attempt 5 — N8n with its own service account:** Discarded. The identity in the calls
would be n8n's, not the Telegram/WA user's. Does not meet the requirement.

**Decision (Attempt 6 — chosen):** N8n forwards the original Telegram/WA body to Mycelium
in downstream calls. Mycelium extracts `from.id`/`wa_id` from the body and resolves the
account — same logic as the webhook endpoint. Trust comes from **source reliability** already
in place: n8n is on the IP allowlist for routes that accept this form of auth.

---

## Consolidated final decisions

### Linking flow

| Platform  | Mechanism          | Endpoint                   | What is stored                                  |
|-----------|--------------------|----------------------------|-------------------------------------------------|
| Telegram  | Mini App initData  | `POST /auth/telegram/link` | `from.id` + `from.username` in `TelegramUser`   |
| WhatsApp  | OTP via WA webhook | `POST /auth/whatsapp/link` | `wa_id` (E.164) in `WhatsAppUser`               |

### Direct login (Telegram only — Plan B, parallel)

- Mini App sends `initData` → `POST /auth/telegram/login`
- Mycelium verifies HMAC, resolves account, issues session
- WhatsApp has no Mini App equivalent — no Plan B for WA

### Gateway passthrough — Leg 1 (platform → n8n)

```
Telegram/WA
    ↓ direct webhook
Mycelium
    → validates platform signature (X-Telegram-Bot-Api-Secret-Token / X-Hub-Signature-256)
    → extracts from.id or wa_id
    → reverse lookup on account.meta (GIN index on JSONB)
    → builds profile
    → routes to n8n via normal gateway pipeline
    → injects profile in headers
    → injects downstream secret (inject_downstream_secret — already exists)
    ↓
n8n
    → validates downstream secret (proves it came from Mycelium)
    → processes with user profile
```

### Gateway passthrough — Leg 2 (n8n → another downstream)

```
n8n
    ↓ forwards original Telegram/WA body
Mycelium
    → source reliability: n8n IP is on the route allowlist (already exists)
    → extracts from.id/wa_id from body
    → resolves account (same logic as webhook endpoint)
    → injects profile and routes normally
    ↓
Another downstream — identity is the user's, not n8n's
```

### New piece in the Route DTO

Optional field `identity_source: Option<IdentitySource>` where `IdentitySource` can be
`Telegram` or `WhatsApp`. When present, `check_security_group` extracts the identity from
the body instead of expecting a JWT. The rest of the pipeline (profile injection, downstream
secret, routing) remains unchanged.

---

## Required infrastructure

| Component                           | Status                    | Location                                          |
|-------------------------------------|---------------------------|---------------------------------------------------|
| `AccountMetaKey::TelegramUser`      | Already exists            | `core/src/domain/dtos/account/meta.rs`            |
| `AccountMetaKey::WhatsAppUser`      | Already exists            | `core/src/domain/dtos/account/meta.rs`            |
| `inject_downstream_secret`          | Already exists            | `ports/api/src/router/inject_downstream_secret.rs`|
| Source reliability (IP allowlist)   | Already exists            | `ports/api/src/router/check_source_reliability.rs`|
| Callback engines (HTTP, Rhai, etc.) | Already exist             | `ports/api/src/callback_engines/`                 |
| GIN index on `account.meta`         | New — migration           | `adapters/diesel/sql/`                            |
| HMAC-SHA256 utility                 | New                       | `lib/http_tools/src/`                             |
| Per-tenant config (bot token, etc.) | New                       | `SecretResolver` + tenant config                  |
| Public webhook endpoints            | New                       | `POST /webhooks/telegram/{tenant_id}`, `/whatsapp/`|
| Identity endpoints                  | New                       | `/auth/telegram/link`, `/login`, `/whatsapp/link` |
| `identity_source` in Route DTO      | New                       | `core/src/domain/dtos/route.rs`                   |
| Branch in `check_security_group`    | New                       | `ports/api/src/router/check_security_group.rs`    |

---

## What remains to be done

- [x] Write the ROADMAP.md entry (next step when resuming)
- [ ] Decide which milestone it fits into (probably M3 — Auth Evolution)
- [ ] Decide whether WA linking via webhook (Mycelium as receiver) or if there is a simpler alternative
