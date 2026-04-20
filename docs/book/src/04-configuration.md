# Configuration

Mycelium reads a single TOML file at startup. Tell it where the file is:

```bash
SETTINGS_PATH=settings/config.toml myc-api
```

---

## Three ways to set a value

Every setting can be defined directly in TOML, via an environment variable, or via Vault:

```toml
# Directly in the file (fine for development)
tokenSecret = "my-secret"

# From an environment variable
tokenSecret = { env = "MYC_TOKEN_SECRET" }

# From HashiCorp Vault (recommended for production)
tokenSecret = { vault = { path = "myc/core/accountLifeCycle", key = "tokenSecret" } }
```

Vault values are resolved at runtime — you don't need to restart after changing a secret in Vault.

---

## What do I actually need to configure?

For a minimal working instance you need:

1. **`[diesel].databaseUrl`** — PostgreSQL connection string.
2. **`[redis]`** — Redis host and optional password.
3. **`[auth].jwtSecret`** — Random string for signing JWT tokens.
4. **`[core.accountLifeCycle].tokenSecret`** — Random string for email verification tokens.
5. **`[smtp]`** — Email server (for magic-link login emails).
6. **`[api].allowedOrigins`** — Allowed CORS origins for your frontend.

Everything else has sensible defaults. See the [Quick Start](./03-quick-start.md) for a copy-pasteable minimal config.

---

## Section reference

### `[vault.define]` — Secret management

Optional. Required only if you use Vault-sourced values anywhere.

```toml
[vault.define]
url = "http://localhost:8200"
versionWithNamespace = "v1/kv"
token = { env = "MYC_VAULT_TOKEN" }
```

| Field | Description |
|---|---|
| `url` | Vault server URL including port |
| `versionWithNamespace` | API version and KV path prefix (e.g. `v1/kv`) |
| `token` | Vault auth token. Use env var or Vault for this value in production |

---

### `[core.accountLifeCycle]` — Identity and email settings

```toml
[core.accountLifeCycle]
domainName = "Mycelium"
domainUrl = "https://mycelium.example.com"
tokenExpiration = 3600
noreplyName = "Mycelium No-Reply"
noreplyEmail = "noreply@example.com"
supportName = "Support"
supportEmail = "support@example.com"
locale = "en-US"
tokenSecret = "random-secret"
```

| Field | Description |
|---|---|
| `domainName` | Human-friendly name shown in emails |
| `domainUrl` | Your frontend URL — used in email links |
| `tokenExpiration` | Email verification token lifetime in seconds |
| `noreplyEmail` | From-address for system emails |
| `supportEmail` | Reply-to address for support |
| `locale` | Email language (e.g. `en-US`, `pt-BR`) |
| `tokenSecret` | Secret for signing email verification tokens |

---

### `[core.webhook]` — Webhook dispatch

```toml
[core.webhook]
acceptInvalidCertificates = true
consumeIntervalInSecs = 10
consumeBatchSize = 10
maxAttempts = 3
```

| Field | Description |
|---|---|
| `acceptInvalidCertificates` | Allow self-signed TLS certs on webhook targets (use `true` in dev only) |
| `consumeIntervalInSecs` | How often to flush the webhook queue |
| `consumeBatchSize` | Events per flush |
| `maxAttempts` | Retry limit per event |

---

### `[diesel]` — Database

```toml
[diesel]
databaseUrl = "postgres://mycelium-user:password@localhost:5432/mycelium-dev"
```

Use Vault for the URL in production:
```toml
databaseUrl = { vault = { path = "myc/database", key = "url" } }
```

---

### `[smtp]` and `[queue]` — Email

```toml
[smtp]
host = "smtp.gmail.com:587"
username = "user@gmail.com"
password = "your-password"

[queue]
emailQueueName = "email-queue"
consumeIntervalInSecs = 5
```

---

### `[redis]` — Cache

```toml
[redis]
protocol = "redis"       # "rediss" for TLS
hostname = "localhost:6379"
password = ""
```

---

### `[auth]` — Authentication

#### Internal login (email + magic link)

```toml
[auth]
internal = "enabled"
jwtSecret = "random-secret"
jwtExpiresIn = 86400     # 24 hours
tmpExpiresIn = 3600      # temporary tokens (password reset, account creation)
```

#### External OAuth2 providers

Add one block per provider:

```toml
# Google
[[auth.external.define]]
issuer = "https://accounts.google.com"
jwksUri = "https://www.googleapis.com/oauth2/v3/certs"
userInfoUrl = "https://www.googleapis.com/oauth2/v3/userinfo"
audience = "your-google-client-id"

# Auth0
[[auth.external.define]]
issuer = "https://your-app.auth0.com/"
jwksUri = "https://your-app.auth0.com/.well-known/jwks.json"
userInfoUrl = "https://your-app.auth0.com/userinfo"
audience = "https://your-app.auth0.com/api/v2/"
```

| Field | Description |
|---|---|
| `issuer` | Provider's identity URL |
| `jwksUri` | URL of the provider's public keys (for JWT verification) |
| `userInfoUrl` | URL to fetch the user's email and claims |
| `audience` | Client ID or API identifier registered with the provider |

---

### `[api]` — Server and routing

```toml
[api]
serviceIp = "0.0.0.0"
servicePort = 8080
serviceWorkers = 4
gatewayTimeout = 30
healthCheckInterval = 120
maxRetryCount = 3
allowedOrigins = ["http://localhost:3000", "https://app.example.com"]

[api.cache]
jwksTtl = 3600     # cache OAuth2 public keys for 1 hour
emailTtl = 120     # cache resolved emails for 2 minutes
profileTtl = 120   # cache resolved profiles for 2 minutes
```

| Field | Description |
|---|---|
| `serviceIp` | Bind address. `0.0.0.0` listens on all interfaces |
| `servicePort` | HTTP port |
| `serviceWorkers` | Worker threads. Match to CPU count |
| `gatewayTimeout` | Request timeout in seconds |
| `allowedOrigins` | CORS whitelist. Use `["*"]` in dev only |
| `healthCheckInterval` | How often to probe downstream health endpoints (seconds) |

---

### `[api.logging]` — Log output

```toml
[api.logging]
level = "info"
format = "ansi"    # "jsonl" for structured logs
target = "stdout"
```

File target:
```toml
target = { file = { path = "logs/api.log" } }
```

OpenTelemetry collector:
```toml
target = { collector = { name = "mycelium-api", host = "otel-collector", protocol = "grpc", port = 4317 } }
```

---

### `[api.tls.define]` — TLS (optional)

```toml
[api.tls.define]
tlsCert = { vault = { path = "myc/api/tls", key = "tlsCert" } }
tlsKey = { vault = { path = "myc/api/tls", key = "tlsKey" } }
```

To disable TLS:
```toml
tls = "disabled"
```

---

### `[api.services]` — Downstream services

Route configuration lives here. See [Downstream APIs](./06-downstream-apis.md) for the full guide.

---

## Next steps

- [Downstream APIs](./06-downstream-apis.md) — Configure routes and security
- [Deploy Locally](./05-deploy-locally.md) — Full environment with Docker Compose
