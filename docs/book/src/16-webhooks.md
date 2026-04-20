# Outbound Webhooks

Mycelium can push notifications to external systems when specific events occur inside the gateway.
This is the **outbound webhook** system — distinct from the Telegram webhook described in
[Alternative Identity Providers](./10-alternative-idps.md), which handles *inbound* calls from
Telegram's servers.

---

## How it works

When an event fires (e.g. a new user account is created), Mycelium delivers a POST request to all
registered webhook URLs that are listening for that event. The delivery runs in the background
and does not block the original operation.

```
Event fires inside Mycelium
  │
  ▼
Webhook dispatcher looks up registered listeners for this event type
  │
  ▼
POST <webhook_url>
Content-Type: application/json
{ "event": "userAccount.created", "payload": { ... } }
```

Delivery is retried up to a configured maximum number of attempts
(`core.webhook.maxAttempts` in `config.toml`).

---

## Configuration

Global webhook delivery settings are in `config.toml` under `[core.webhook]`:

```toml
[core.webhook]
acceptInvalidCertificates = false    # set true in dev with self-signed certs
consumeIntervalInSecs = 30           # how often the dispatcher polls for pending deliveries
consumeBatchSize = 25                # how many deliveries to process per poll cycle
maxAttempts = 5                      # retry limit before marking a delivery as failed
```

---

## Registering a webhook

Webhooks are managed through the `systemManager.webhooks.*` JSON-RPC methods or the equivalent
REST routes under `/_adm/system-manager/webhooks/`. Requires the **system-manager** role.

### Via JSON-RPC

```json
{
  "jsonrpc": "2.0",
  "method": "systemManager.webhooks.create",
  "params": {
    "url": "https://notify.internal/mycelium-events",
    "trigger": "userAccount.created",
    "isActive": true
  },
  "id": 1
}
```

### Via REST

```http
POST /_adm/system-manager/webhooks
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "url": "https://notify.internal/mycelium-events",
  "trigger": "userAccount.created",
  "isActive": true
}
```

---

## Event types

| Event | Fires when |
|---|---|
| `subscriptionAccount.created` | A new subscription account is created within a tenant |
| `subscriptionAccount.updated` | A subscription account's name or flags are changed |
| `subscriptionAccount.deleted` | A subscription account is deleted |
| `userAccount.created` | A new personal user account is registered |
| `userAccount.updated` | A user account's name is changed |
| `userAccount.deleted` | A user account is deleted |

---

## Managing webhooks

| JSON-RPC method | REST path | Description |
|---|---|---|
| `systemManager.webhooks.create` | `POST /_adm/system-manager/webhooks` | Register a new webhook |
| `systemManager.webhooks.list` | `GET /_adm/system-manager/webhooks` | List registered webhooks |
| `systemManager.webhooks.update` | `PATCH /_adm/system-manager/webhooks/{id}` | Update URL, trigger, or active status |
| `systemManager.webhooks.delete` | `DELETE /_adm/system-manager/webhooks/{id}` | Remove a webhook |

---

## Delivery payload

Each POST to the registered URL includes a JSON body with the event type and a payload
describing what changed. The exact payload shape depends on the event type.

Example — `userAccount.created`:

```json
{
  "event": "userAccount.created",
  "occurredAt": "2026-04-20T14:30:00Z",
  "payload": {
    "accountId": "a1b2c3d4-...",
    "email": "alice@example.com",
    "name": "Alice"
  }
}
```

---

## Security

Webhook URLs should use HTTPS. Mycelium does not add a signature header to outbound webhook
calls by default. If your endpoint needs to verify the source, place it behind a route that
requires a secret (see [Downstream APIs](./06-downstream-apis.md#authenticating-mycelium-to-your-service-secrets)).

To accept self-signed certificates during development, set
`acceptInvalidCertificates = true` in `[core.webhook]`.
