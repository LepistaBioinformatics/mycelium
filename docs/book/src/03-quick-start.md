# Quick Start Guide

This guide will help you get Mycelium API Gateway up and running in minutes with a minimal configuration.

## Prerequisites

Before starting, ensure you have completed the [Installation Guide](./02-installation.md) and have:
- Mycelium API Gateway installed
- Postgres database initialized
- Redis server running

## Step 1: Prepare the Configuration File

Mycelium uses a TOML configuration file to define its settings. Create a minimal configuration file or use the example provided in the repository.

### Using the Example Configuration

If you built from source, copy the example configuration:

```bash
cp settings/config.example.toml settings/config.toml
```

### Minimal Configuration

Create a new file `settings/config.toml` with the following minimal content:

```toml
[core.accountLifeCycle]
domainName = "Mycelium Dev"
domainUrl = "http://localhost:8080"
tokenExpiration = 3600
noreplyName = "Mycelium No-Reply"
noreplyEmail = "noreply@example.com"
supportName = "Mycelium Support"
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

**Important:** Replace the following placeholders:
- `YOUR_DB_PASSWORD` - The password you set during database initialization
- `your-secret-key-change-me-in-production` - A strong secret for token encryption
- `your-jwt-secret-change-me-in-production` - A strong secret for JWT signing

## Step 2: Define Routes in Configuration

Routes are now defined directly in the main configuration file using TOML's array of tables syntax. Add the following to your `config.toml`:

```toml
# ------------------------------------------------------------------------------
# SERVICE CONFIGURATIONS
# ------------------------------------------------------------------------------
[api.services]

# Example downstream service (optional)
[[my-service]]
host = "localhost:3000"
protocol = "http"

[[my-service.path]]
group = "public"
path = "/api/*"
methods = ["GET", "POST", "PUT", "DELETE"]
```

## Step 3: Start Mycelium

Now you can start the Mycelium API Gateway:

### If installed via Cargo:
```bash
SETTINGS_PATH=settings/config.toml myc-api
```

### If built from source:
```bash
SETTINGS_PATH=settings/config.toml ./target/release/myc-api
```

### If using Docker:
```bash
docker run -d \
  --name mycelium-api \
  -p 8080:8080 \
  -v $(pwd)/settings:/app/settings \
  -e SETTINGS_PATH=settings/config.toml \
  sgelias/mycelium-api:latest
```

## Step 4: Verify the Gateway is Running

Test the health check endpoint:

```bash
curl http://localhost:8080/health
```

You should receive a response indicating the gateway is healthy.

## Step 5: Test Authentication (Optional)

To test the internal authentication system, you'll need to:

1. Create a user account
2. Verify the email (or skip verification in development)
3. Login to receive a JWT token

For detailed authentication workflows, see the [Authorization Guide](./01-authorization.md).

## Common Quick Start Commands

### Check Logs
If you're running in the foreground, logs will appear in the terminal. For Docker:

```bash
docker logs -f mycelium-api
```

### Stop the Gateway

Press `Ctrl+C` if running in the foreground, or for Docker:

```bash
docker stop mycelium-api
```

### Restart with Configuration Changes

After modifying the configuration file, restart the gateway:

```bash
# Kill the current process and restart
SETTINGS_PATH=settings/config.toml myc-api
```

## Environment Variables

Mycelium supports configuration through environment variables. You can override any TOML setting:

```bash
# Set database URL via environment variable
export DATABASE_URL="postgres://user:pass@localhost:5432/mycelium"
SETTINGS_PATH=settings/config.toml myc-api
```

For more details on environment variable configuration, see the [Configuration Guide](./04-configuration.md).

## Next Steps

Now that you have Mycelium running, you can:

- **Configure Downstream Services**: Learn how to [configure routes](./06-downstream-apis.md) to proxy to your backend services
- **Set Up Authentication**: Configure [OAuth2 providers](./04-configuration.md#external-authentication) or use internal authentication
- **Enable Vault**: Secure your secrets with [HashiCorp Vault](./04-configuration.md#vault-configurations)
- **Deploy to Production**: See the [deployment guides](./05-deploy-locally.md) for production-ready setups
- **Run Tests**: Learn how to [run the test suite](./07-running-tests.md)

## Troubleshooting

### Gateway won't start

**Check the configuration file syntax:**
```bash
# Validate TOML syntax
cat settings/config.toml | grep -v "^#" | grep -v "^$"
```

**Check database connectivity:**
```bash
psql postgres://mycelium-user:YOUR_PASSWORD@localhost:5432/mycelium-dev -c "SELECT 1"
```

**Check Redis connectivity:**
```bash
redis-cli ping
```

### Port already in use

If port 8080 is already in use, change the `servicePort` in `config.toml`:

```toml
[api]
servicePort = 8081  # Use a different port
```

### Cannot connect to downstream services

Ensure:
1. The downstream service is running
2. The host and port in `config.toml` are correct
3. The protocol (http/https) matches the downstream service

For more help, visit the [GitHub Issues](https://github.com/LepistaBioinformatics/mycelium/issues) page.
