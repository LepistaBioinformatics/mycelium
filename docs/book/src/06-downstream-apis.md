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

### `protectedByServiceTokenWithRole` — Service-to-service authentication

For machine-to-machine calls. The caller must present a service token with the right role.

```toml
[[my-service.path]]
group = { protectedByServiceTokenWithRole = { roles = ["service-admin"] } }
path = "/internal/*"
methods = ["ALL"]
```

Service token format:
```
tid=<uuid>;rid=<uuid>;r=<role>;edt=<datetime>;sig=<hmac>
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
| `protectedByServiceToken*` | No | Yes |

The `x-mycelium-profile` value is a Base64-encoded, ZSTD-compressed JSON object. Use the
[Python SDK](https://github.com/LepistaBioinformatics/mycelium-sdk-py) or decode it manually
to read tenant memberships, roles, and access scopes.

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
