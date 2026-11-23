# Downstream APIs

This guide explains how to register a backend service with Mycelium and control who can access it.

All service configuration lives in `settings/config.toml` under `[api.services]`. Restart the
gateway after any change.

---

## Registering a service

The simplest possible registration:

```toml
[api.services]

[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.path]]
group = "public"
path = "/api/*"
methods = ["GET", "POST"]
```

This tells Mycelium: "Any GET or POST to `/api/*` should be forwarded to `localhost:3000`."
No authentication required — anyone can reach this route.

The service name (`my-service`) becomes part of how you identify the service internally. It
does not affect the URL path; the path comes from the `path` field.

---

## Choosing a security group

The `group` field on each route controls what Mycelium requires before forwarding the request.

### `public` — No authentication

Anyone can access the route. Use for health checks, public APIs, and Telegram webhooks.

```toml
[[my-service.path]]
group = "public"
path = "/health"
methods = ["GET"]
```

### `authenticated` — Valid login token required

The user must be logged in. Mycelium injects their email in `x-mycelium-email`.

```toml
[[my-service.path]]
group = "authenticated"
path = "/profile"
methods = ["GET", "PUT"]
```

### `protected` — Full identity required

The user must be logged in and have a resolved profile. Mycelium injects the full profile
in `x-mycelium-profile`. Use this when your service needs to make fine-grained decisions
(e.g. "can this user access this specific resource?").

```toml
[[my-service.path]]
group = "protected"
path = "/dashboard/*"
methods = ["GET"]
```

### `protectedByRoles` — Specific roles required

Only users with at least one of the listed roles can pass. All others get 403.

```toml
[[my-service.path]]
group = { protectedByRoles = [{ slug = "admin" }, { slug = "super-admin" }] }
path = "/admin/*"
methods = ["ALL"]
```

You can also require a specific permission level:

```toml
[[my-service.path]]
group = { protectedByRoles = [{ slug = "editor", permission = "write" }] }
path = "/content/edit/*"
methods = ["POST", "PUT", "DELETE"]

[[my-service.path]]
group = { protectedByRoles = [{ slug = "viewer", permission = "read" }] }
path = "/content/view/*"
methods = ["GET"]
```

---

## Multiple routes on one service

Each `[[service-name.path]]` block adds a route. Mix security groups freely:

```toml
[[user-service]]
host = "users.internal:4000"
protocol = "http"

[[user-service.path]]
group = "authenticated"
path = "/users/me"
methods = ["GET", "PUT"]

[[user-service.path]]
group = "protected"
path = "/users/preferences"
methods = ["GET", "POST"]

[[user-service.path]]
group = { protectedByRoles = [{ slug = "admin" }] }
path = "/users/admin/*"
methods = ["ALL"]
```

---

## Authenticating Mycelium to your service (secrets)

If your downstream service requires a token or API key from the caller, define a secret and
reference it on the route.

**Query parameter:**
```toml
[[legacy-api]]
host = "legacy.internal:8080"
protocol = "http"

[[legacy-api.secret]]
name = "api-key"
queryParameter = { name = "token", token = { env = "LEGACY_API_KEY" } }

[[legacy-api.path]]
group = "public"
path = "/legacy/*"
methods = ["GET"]
secretName = "api-key"
```

**Authorization header:**
```toml
[[protected-api.secret]]
name = "bearer-token"
authorizationHeader = { name = "Authorization", prefix = "Bearer ", token = { vault = { path = "myc/services/api", key = "token" } } }
```

---

## Load balancing across multiple hosts

```toml
[[api-service]]
hosts = ["api-01.example.com:8080", "api-02.example.com:8080"]
protocol = "https"

[[api-service.path]]
group = "protected"
path = "/api/*"
methods = ["ALL"]
```

---

## Retrying transient failures

A route can retry requests that fail with a connection error or with one of
the configured statuses. Retries are off unless the route sets `retry`:

```toml
[[api-service.path]]
group = "protected"
path = "/api/reports/*"
methods = ["GET", "PUT"]
retry = { maxAttempts = 3, baseDelayMs = 100, maxDelayMs = 2000, retryOnStatus = [502, 503, 504] }
```

| Key | Default | Description |
|---|---|---|
| `maxAttempts` | `3` | Total attempts, including the first one (1 to 10) |
| `baseDelayMs` | `100` | Base of the back-off. The n-th retry waits `min(baseDelayMs * n², maxDelayMs)` |
| `maxDelayMs` | `2000` | Upper bound of a single wait. Must be at least `baseDelayMs` |
| `retryOnStatus` | `[502, 503, 504]` | Downstream statuses that trigger a retry. Only `429` and `5xx` are accepted |
| `retryNonIdempotent` | `false` | Also retry `POST`, `PATCH` and `CONNECT` |

Things to keep in mind:

- Only idempotent methods (`GET`, `HEAD`, `OPTIONS`, `PUT`, `DELETE`,
  `TRACE`) are retried by default. With `retryNonIdempotent = true`, the other
  methods are retried on connection failures and configured statuses, but
  never after a timeout or a broken response, since the service may already
  have acted on the request.
- The request body of a retried route is held in memory so it can be sent
  again. Bodies above 512 KB are rejected with `400`.
- When every attempt fails, the client receives the last status or error.
  Response callbacks run once, on that final result.
- The worst-case latency is `maxAttempts` times the gateway timeout, plus the
  back-off delays.

---

## Webhook routes — identity from request body

Some callers (like Telegram) don't send a JWT. Instead, the user's identity is in the request
body. Use `identitySource` to handle this.

**Requires `allowedSources`** — before parsing the body, Mycelium checks that the `Host`
header matches an allowed source. This prevents attackers from forging webhook calls.

```toml
[[telegram-bot]]
host = "bot-service:3000"
protocol = "http"
allowedSources = ["api.telegram.org"]

[[telegram-bot.path]]
group = "protected"
path = "/telegram/webhook"
methods = ["POST"]
identitySource = "telegram"
```

With this config, Mycelium extracts the Telegram user ID from the message body, looks up
the linked Mycelium account, and injects `x-mycelium-profile` before forwarding. If the
user hasn't linked their account, Mycelium returns 401 and the message is not forwarded.

See [Alternative Identity Providers](./10-alternative-idps.md) for the full Telegram setup journey.

---

## Service discovery (AI agents)

Set `discoverable = true` to make a service visible to AI agents and LLM-based tooling:

```toml
[[data-service]]
host = "data.internal:5000"
protocol = "http"
discoverable = true
description = "Customer data API"
openapiPath = "/api/openapi.json"
healthCheckPath = "/health"
capabilities = ["customer-search", "order-history"]
serviceType = "rest-api"
```

---

## What headers does my service receive?

| Security group | `x-mycelium-email` | `x-mycelium-profile` |
|---|---|---|
| `authenticated` | Yes | No |
| `protected` | Yes | Yes |
| `protectedByRoles` | Yes | Yes |

The `x-mycelium-profile` value is a Base64-encoded, ZSTD-compressed JSON object. Use the
[Python SDK](https://github.com/LepistaBioinformatics/mycelium-sdk-py) or decode it manually
to read tenant memberships, roles, and access scopes.
Grants the user holds on linked peer instances travel apart, under `federatedResources`; the SDK
filters ignore them unless asked (see [Federated Grants](./29-federated-grants.md#in-the-profile)).

Every request that carries `x-mycelium-profile` also carries `x-mycelium-profile-signature`, which lets your service prove
the profile came from the gateway. See [Downstream Profile Signature](./27-downstream-profile-signature.md).

---

## Complete example

```toml
[api.services]

# Public health check
[[health]]
host = "localhost:8080"
protocol = "http"

[[health.path]]
group = "public"
path = "/health"
methods = ["GET"]

# User service — mixed access levels
[[user-service]]
host = "users.internal:4000"
protocol = "http"

[[user-service.path]]
group = "authenticated"
path = "/users/me"
methods = ["GET", "PUT"]

[[user-service.path]]
group = { protectedByRoles = [{ slug = "admin" }] }
path = "/users/admin/*"
methods = ["ALL"]

# Telegram webhook
[[support-bot]]
host = "bot.internal:3000"
protocol = "http"
allowedSources = ["api.telegram.org"]

[[support-bot.path]]
group = "protected"
path = "/telegram/webhook"
methods = ["POST"]
identitySource = "telegram"
```

---

## Sending emails from a service (notifications)

A downstream service can ask the gateway to send an email on its behalf, for
example from a background job that has no user session. The gateway renders an
operator-deployed template, sets the sender and the attribution itself, and
queues one message per recipient.

The feature is off for every service until the service declares a
`notifications` block:

```toml
[core.serviceNotifications]
templatesDir = "service-templates"   # required once any service enables notifications
# senderTagMode = "header"           # header | plus
# maxConnectionStringTtlInDays = 90
# retentionDays = 90

[api.services]

[[service-01]]
host = "service-01.internal:8080"
protocol = "http"

[service-01.notifications]
senderTag = "service-01"
displayName = "Service 01"
issuerRoles = ["service-01-admin"]
recipientPolicy = "owner"            # owner | tenantMembers | domains | any
templates = ["welcome"]
quotaPerHour = 100                   # shared by every gateway pod
maxRecipientsPerMessage = 10         # must not exceed quotaPerHour
```

Templates live at `<templatesDir>/<senderTag>/<locale>/<name>.jinja` (body) and
`<name>.subject`. Every declared template must exist for `en-us`. A missing or
malformed template stops the gateway at boot.

**Issuing a string.** A user holding one of the `issuerRoles` calls
`POST /_adm/beginners/tokens/notifications` (or the JSON-RPC method
`beginners.tokens.createNotification`) with the service `senderTag`. The
returned connection string works only on the send endpoint: every other surface
rejects it.

`beginners.tokens.listNotificationServices` returns the services the caller can
issue for, the tenants each string can be bound to and the instance
`maxConnectionStringTtlInDays`, so a UI can offer only valid choices.

**Sending.** The service calls `POST /_adm/notifications/email` with the string
in the `x-mycelium-connection-string` header:

```json
{ "to": ["user@example.com"], "template": "welcome", "variables": { "name": "Ana" }, "locale": "pt-br" }
```

- The body accepts only these four fields. The service cannot choose the
  sender, the reply address, the subject, headers or raw HTML.
- `variables` holds strings, numbers and booleans only, at most 8192 bytes as
  compact JSON.
- Each `to` entry must be one bare address.
- The gateway re-checks the string creator on every send. If the account was
  deactivated, the role revoked or the service disabled, the send is refused.
- Answers: `202` with one message id per recipient; `401` for a missing,
  tampered, expired, revoked or general string; `403` for a disabled service,
  an issuer no longer allowed or a recipient outside the policy; `422` for an
  invalid body; `429` with `Retry-After` when the hourly quota is spent.

> **The connection string is a long-lived secret.** Store it like a password
> and never log it.

### Operator notes

- **Rollout gate.** Add the first `notifications` block only after every
  gateway pod runs a version that supports it. Older pods answer `401` to
  notification strings while the rollout is in progress.
- **Rollback rule.** Before you roll back to a version without this feature,
  delete every notification connection string. An older binary cannot read
  them, and it fails to list the connection strings of the accounts that own
  them.
- **Sender tag mode.** `header` (the default) sends from `noreplyEmail`
  unchanged and carries the service in the `X-Mycelium-Source-Service`
  header. It works with every provider. `plus` sends from
  `<local>+<senderTag>@<domain>`. Turn it on only after you confirm that your
  provider accepts those senders:
  - **Google SMTP** (`smtp.gmail.com`) rejects or rewrites a `From` that is
    not the authenticated account or a configured "send as" alias. Each
    `+tag` address needs an alias, or use the SMTP relay
    (`smtp-relay.gmail.com`) configured for the domain.
  - **Azure Communication Services Email** accepts only `MailFrom` usernames
    registered on the verified domain, so each `<noreply>+<tag>` must be
    registered.

  Neither provider has been tested with `plus`. When in doubt, keep `header`:
  the display name and the `X-Mycelium-*` headers carry the attribution in
  both modes.
- **Retention.** The dispatch trace (`notification_dispatch`) records who sent
  what to whom, without the template variables. Rows older than
  `retentionDays` are purged at most once an hour.

---

## Troubleshooting

**Route not matching** — Check that the path has a wildcard (`/api/*`) if you want to match
subpaths. `/api/` only matches that exact path.

**401 on a protected route** — The JWT or connection string is missing, expired, or invalid.
Check the `Authorization: Bearer <token>` or `x-mycelium-connection-string` header.

**403 on a role-protected route** — The user is authenticated but doesn't have the required role.
Check role assignment in the Mycelium admin panel.

**Downstream unreachable** — Verify `host`, `protocol`, and that the service is actually running.
Use `acceptInsecureRouting = true` on the route if the downstream uses a self-signed TLS cert.

---

## Reference — service-level fields

| Field | Required | Description |
|---|---|---|
| `host` | Yes (or `hosts`) | Single downstream host with port |
| `hosts` | Yes (or `host`) | Multiple hosts for load balancing |
| `protocol` | Yes | `"http"` or `"https"` |
| `allowedSources` | Required when `identitySource` is set | Allowed `Host` headers (supports wildcards) |
| `discoverable` | No | Expose service to AI agents |
| `description` | No | Human-readable description |
| `openapiPath` | No | Path to OpenAPI spec |
| `healthCheckPath` | No | Health check endpoint |
| `capabilities` | No | Array of capability tags |
| `serviceType` | No | e.g. `"rest-api"` |

## Reference — route-level fields

| Field | Required | Description |
|---|---|---|
| `group` | Yes | Security group (see above) |
| `path` | Yes | URL path pattern, supports wildcards |
| `methods` | Yes | HTTP methods, or `["ALL"]` |
| `secretName` | No | Reference to a secret defined at service level |
| `identitySource` | No | Body-based IdP. Currently: `"telegram"` |
| `acceptInsecureRouting` | No | Allow self-signed TLS certs on downstream |
| `retry` | No | Retry policy for transient failures (see above) |
