# Quick Start

This guide gets Mycelium running with a minimal configuration. By the end you'll have a
gateway that can route requests to a downstream service.

**Before starting:** complete the [Installation](./02-installation.md) guide — you need
PostgreSQL running, Redis running, and the `myc-api` binary installed.

---

## Step 1 — Create a configuration file

Mycelium reads a single TOML file. Copy the example from the repository or create `settings/config.toml` with the content below.

> Replace `YOUR_DB_PASSWORD` with the password you set during database setup.
> Replace both `your-secret-*` values with random strings (use `openssl rand -hex 32`).

```toml
[core.accountLifeCycle]
domainName = "My App"
domainUrl = "http://localhost:8080"
tokenExpiration = 3600
noreplyName = "No-Reply"
noreplyEmail = "noreply@example.com"
supportName = "Support"
supportEmail = "support@example.com"
locale = "en-US"
tokenSecret = "your-secret-key-change-me-in-production"

[core.webhook]
acceptInvalidCertificates = true
consumeIntervalInSecs = 10
consumeBatchSize = 10
maxAttempts = 3

[diesel]
databaseUrl = "postgres://mycelium-user:YOUR_DB_PASSWORD@localhost:5432/mycelium-dev"

[smtp]
host = "smtp.example.com:587"
username = "user@example.com"
password = "your-smtp-password"

[queue]
emailQueueName = "email-queue"
consumeIntervalInSecs = 5

[redis]
protocol = "redis"
hostname = "localhost:6379"
password = ""

[auth]
internal = "enabled"
jwtSecret = "your-jwt-secret-change-me-in-production"
jwtExpiresIn = 86400
tmpExpiresIn = 3600

[api]
serviceIp = "0.0.0.0"
servicePort = 8080
serviceWorkers = 4
gatewayTimeout = 30
healthCheckInterval = 120
maxRetryCount = 3
allowedOrigins = ["*"]

[api.cache]
jwksTtl = 3600
emailTtl = 120
profileTtl = 120

[api.logging]
level = "info"
format = "ansi"
target = "stdout"
```

---

## Step 2 — Start the gateway

```bash
SETTINGS_PATH=settings/config.toml myc-api
```

With Docker:
```bash
docker run -d \
  --name mycelium-api \
  -p 8080:8080 \
  -v $(pwd)/settings:/app/settings \
  -e SETTINGS_PATH=settings/config.toml \
  sgelias/mycelium-api:latest
```

---

## Step 3 — Verify it's running

```bash
curl http://localhost:8080/health
```

A successful response means the gateway is up and connected to the database and Redis.

---

## Step 4 — Register your first downstream service (optional)

Add this block to your `config.toml` to proxy a backend service:

```toml
[api.services]

[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.path]]
group = "public"
path = "/api/*"
methods = ["GET", "POST", "PUT", "DELETE"]
```

Restart the gateway after any config change.

---

## Next steps

- [Configuration](./04-configuration.md) — Understand every config option
- [Downstream APIs](./06-downstream-apis.md) — Add authentication and role checks to your routes
- [Deploy Locally](./05-deploy-locally.md) — Full Docker Compose environment with all dependencies
- [Alternative Identity Providers](./10-alternative-idps.md) — Add Telegram or OAuth2 login

---

## Troubleshooting

**Gateway won't start** — Check TOML syntax, then verify database and Redis connectivity:
```bash
psql postgres://mycelium-user:YOUR_PASSWORD@localhost:5432/mycelium-dev -c "SELECT 1"
redis-cli ping
```

**Port 8080 already in use** — Change `servicePort` in `config.toml`.
