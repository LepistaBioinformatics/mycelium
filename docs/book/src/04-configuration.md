# Configuration Guide

This document describes the configurations of the TOML file used to set up the Mycelium API Gateway. An example configuration file can be found at `settings/config.example.toml` in the repository.

## Configuration File Location

Mycelium reads its configuration from a TOML file specified via the `SETTINGS_PATH` environment variable:

```bash
SETTINGS_PATH=settings/config.toml myc-api
```

## Configuration Methods

Mycelium provides three different ways to configure settings:

1. **Directly in the TOML file** (for development purposes)
2. **Environment variables** (for production purposes)
3. **Externally in a Vault server** (for production, highly recommended)

All three options are valid for any configuration, but we recommend using Vault for storing secrets in production environments.

### Example: Three Ways to Configure

**Using directly in the TOML file:**
```toml
[core.accountLifeCycle]
tokenSecret = "my-secret"
```

**Using environment variables:**
```toml
[core.accountLifeCycle]
tokenSecret = { env = "MYC_TOKEN_SECRET" }
```

Then set the environment variable:
```bash
export MYC_TOKEN_SECRET="my-secret"
```

**Using Vault:**
```toml
[core.accountLifeCycle]
tokenSecret = { vault = { path = "myc/core/accountLifeCycle", key = "tokenSecret" } }
```

> **Security Best Practice**: Mycelium will try to resolve the variables at runtime, so it is not necessary to restart the API Gateway after changing the configuration in Vault.

## 1. Vault Configurations (`vault`)

Highly recommended for production environments.

If you opt to use the Vault server to store secrets, configure the following options:

```toml
[vault.define]
url = "http://localhost:8200"
versionWithNamespace = "v1/kv"
token = "your-vault-token"
```

**Configuration Options:**

- **`url`**: The URL of the Vault server. Should include the protocol, hostname, and port number if necessary.
- **`versionWithNamespace`**: The API version used to interact with the Vault server. Example: `v1/kv`.
- **`token`**: The token used to authenticate with the Vault server. For obvious reasons, this should be configured using environment variables in production.

**Example with environment variable:**
```toml
[vault.define]
url = "http://localhost:8200"
versionWithNamespace = "v1/kv"
token = { env = "VAULT_TOKEN" }
```

## 2. Core Configurations (`core`)

Here resides the core configurations of the API Gateway. Configs defined here should be used to configure the basic application lifecycle and webhooks.

### 2.1 Account Lifecycle (`accountLifeCycle`)

```toml
[core.accountLifeCycle]
domainName = "Mycelium"
domainUrl = "https://mycelium.com"
tokenExpiration = 3600
noreplyName = "Mycelium No-Reply"
noreplyEmail = "noreply@mycelium.com"
supportName = "Mycelium Support"
supportEmail = "support@mycelium.com"
locale = "en-US"
tokenSecret = "your-secret-key"
```

**Configuration Options:**

- **`domainName`**: The human-friendly name of the domain. Usually this is the frontend domain name. Example: `Mycelium`.
- **`domainUrl`**: The URL of the domain. Example: `https://mycelium.com`.
- **`tokenExpiration`**: Token expiration time in seconds.
- **`noreplyName`** / **`noreplyEmail`**: Name and email for automatic messages.
- **`supportName`** / **`supportEmail`**: Name and email for support.
- **`locale`**: Default locale. Emails will be sent in this locale.
- **`tokenSecret`**: A unique secret used to encrypt JWT tokens.

### 2.2 Webhook (`webhook`)

```toml
[core.webhook]
acceptInvalidCertificates = true
consumeIntervalInSecs = 10
consumeBatchSize = 10
maxAttempts = 3
```

**Configuration Options:**

- **`acceptInvalidCertificates`**: Allows self-signed certificates (useful for development).
- **`consumeIntervalInSecs`**: The interval in seconds between each batch of webhook dispatch events.
- **`consumeBatchSize`**: The number of events processed per batch.
- **`maxAttempts`**: The maximum number of attempts to process an event.

## 3. SQL Database Adapter Settings (`diesel`)

```toml
[diesel]
databaseUrl = "postgres://mycelium-user:password@localhost:5432/mycelium-dev"
```

**Configuration Options:**

- **`databaseUrl`**: The database URL. This is highly recommended to be stored in Vault or as an environment variable.

**Example with Vault:**
```toml
[diesel]
databaseUrl = { vault = { path = "myc/database", key = "url" } }
```

## 4. Notifier Adapter Settings (`smtp` and `queue`)

### 4.1 SMTP (`smtp`)

```toml
[smtp]
host = "smtp.gmail.com:587"
username = "user@gmail.com"
password = "your-password"
```

**Configuration Options:**

- **`host`**: The SMTP server address including port.
- **`username`** / **`password`**: SMTP credentials.

### 4.2 Email Queue (`queue`)

```toml
[queue]
emailQueueName = "email-queue"
consumeIntervalInSecs = 5
```

**Configuration Options:**

- **`emailQueueName`**: The name of the email queue.
- **`consumeIntervalInSecs`**: The interval in seconds between each batch of email dispatch events.

## 5. Redis Settings (`redis`)

Redis is used to store cache and session data.

```toml
[redis]
protocol = "redis"
hostname = "localhost:6379"
password = ""
```

**Configuration Options:**

- **`protocol`**: The protocol used to connect to the Redis server (usually `redis` or `rediss` for secure connection).
- **`hostname`**: The address of the Redis server including port.
- **`password`**: The password used to connect to the Redis server (leave empty if no password).

## 6. Authentication Settings (`auth`)

Mycelium provides a flexible authentication system that allows you to configure internal and external authentication providers.

### 6.1 Internal Authentication (`internal`)

Internal authentication is the default authentication provider and is used to authenticate users against the Mycelium database.

```toml
[auth]
internal = "enabled"
jwtSecret = "your-jwt-secret"
jwtExpiresIn = 86400
tmpExpiresIn = 3600
```

**Configuration Options:**

- **`internal`**: Set to `"enabled"` to enable internal authentication.
- **`jwtSecret`**: A unique secret used to encrypt JWT tokens.
- **`jwtExpiresIn`**: JWT expiration time in seconds (default: 86400 = 24 hours).
- **`tmpExpiresIn`**: Temporary token expiration time in seconds. This is used to generate temporary tokens during password reset and account creation processes.

### 6.2 External Authentication (`external`)

External authentication is used to authenticate users against any external provider that supports OAuth 2.0.

```toml
[[auth.external.define]]
issuer = "https://accounts.google.com"
jwksUri = "https://www.googleapis.com/oauth2/v3/certs"
userInfoUrl = "https://www.googleapis.com/oauth2/v3/userinfo"
audience = "your-google-client-id"
```

**Configuration Options:**

- **`issuer`**: The issuer of the OAuth 2.0 provider. This is a URL that identifies the provider.
- **`jwksUri`**: The URI of the JWKS endpoint. This is a URL that points to the JWKS (JSON Web Key Set) endpoint of the provider.
- **`userInfoUrl`**: The URI of the user info endpoint. This is a URL that points to the user info endpoint of the provider.
- **`audience`**: The audience of the OAuth 2.0 provider. This is a string that identifies the API that the provider is used for.

**Example with Multiple Providers:**
```toml
# Google OAuth2
[[auth.external.define]]
issuer = "https://accounts.google.com"
jwksUri = "https://www.googleapis.com/oauth2/v3/certs"
userInfoUrl = "https://www.googleapis.com/oauth2/v3/userinfo"
audience = "your-google-client-id"

# Microsoft
[[auth.external.define]]
issuer = "https://sts.windows.net/{tenantId}/"
jwksUri = "https://login.microsoftonline.com/{tenantId}/discovery/keys"
userInfoUrl = "https://graph.microsoft.com/oidc/userinfo"
audience = "00000003-0000-0000-c000-000000000000"

# Auth0
[[auth.external.define]]
issuer = "https://{app-name}.auth0.com/"
jwksUri = "https://{app-name}.auth0.com/.well-known/jwks.json"
userInfoUrl = "https://{app-name}.auth0.com/userinfo"
audience = "https://{app-name}.auth0.com/api/v2/"
```

## 7. API Settings (`api`)

### 7.1 Service Settings

```toml
[api]
serviceIp = "0.0.0.0"
servicePort = 8080
serviceWorkers = 4
gatewayTimeout = 30
healthCheckInterval = 120
maxRetryCount = 3

allowedOrigins = [
    "http://localhost:8080",
    "https://localhost:8080",
    "http://localhost:3000"
]

[api.cache]
jwksTtl = 3600
emailTtl = 120
profileTtl = 120
```

**Configuration Options:**

- **`serviceIp`**: Service IP address. Usually `0.0.0.0` to listen on all interfaces.
- **`servicePort`**: Service port. Usually `8080`.
- **`serviceWorkers`**: Number of worker threads. Usually `4` or number of CPU cores.
- **`gatewayTimeout`**: Gateway timeout in seconds. Usually `30`.
- **`allowedOrigins`**: Array of allowed CORS origins. Use `["*"]` for development, specify exact origins for production.
- **`healthCheckInterval`**: Health check interval in seconds. Usually `120`.
- **`maxRetryCount`**: Maximum retry count for failed requests. Usually `3`.
- **`cache.jwksTtl`**: JWKS cache TTL in seconds. Usually `3600` (1 hour).
- **`cache.emailTtl`**: Email cache TTL in seconds. Usually `120`.
- **`cache.profileTtl`**: Profile cache TTL in seconds. Usually `120`.

### 7.2 Logging Settings (`logging`)

```toml
[api.logging]
level = "info"
format = "ansi"
target = "stdout"
```

**Configuration Options:**

- **`level`**: Log level. Options: `trace`, `debug`, `info`, `warn`, `error`. Can be set globally or per module.
- **`format`**: Log format. Options: `jsonl` (JSON Lines), `ansi` (colored terminal output).
- **`target`**: Log destination. Options: `stdout`, `file`, or `collector` (for Jaeger).

**Example with Per-Module Log Levels:**
```toml
[api.logging]
level = "mycelium_base=trace,myc_api=debug,actix_web=warn"
format = "jsonl"
target = "stdout"
```

**Example with File Target:**
```toml
[api.logging]
level = "info"
format = "jsonl"
target = { file = { path = "logs/api.log" } }
```

**Example with Jaeger/OpenTelemetry Collector:**
```toml
[api.logging]
level = "info"
format = "jsonl"
target = { collector = { name = "mycelium-api", host = "otel-collector", protocol = "grpc", port = 4317 } }
```

### 7.3 TLS Settings (`tls`)

```toml
[api.tls.define]
tlsCert = '''
-----BEGIN CERTIFICATE-----
...
-----END CERTIFICATE-----
'''
tlsKey = '''
-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----
'''
```

**Configuration Options:**

- **`tlsCert`** / **`tlsKey`**: TLS certificates. Use triple quotes (`'''`) for multi-line strings.

**Example with Vault:**
```toml
[api.tls.define]
tlsCert = { vault = { path = "myc/api/tls", key = "tlsCert" } }
tlsKey = { vault = { path = "myc/api/tls", key = "tlsKey" } }
```

To disable TLS:
```toml
tls = "disabled"
```

### 7.4 Service Configurations

Services and routes are now configured directly in the main TOML file. See the [Downstream APIs Configuration](./06-downstream-apis.md) for detailed examples.

## Complete Configuration Example

For a complete example with all options, see `settings/config.example.toml` in the repository:

```bash
cat settings/config.example.toml
```

## Environment Variables Reference

You can override any configuration using environment variables. The environment variable name should match the TOML path using underscores and uppercase letters:

```bash
# Override service port
export API_SERVICE_PORT=8081

# Override database URL
export DIESEL_DATABASE_URL="postgres://user:pass@localhost/db"

# Override JWT secret
export AUTH_JWT_SECRET="my-secret"
```

## Validation

Mycelium validates the configuration at startup. If there are errors, they will be displayed in the logs with suggestions for fixing them.

To validate your configuration without starting the server:

```bash
# This will validate and exit
SETTINGS_PATH=settings/config.toml myc-api --validate
```

## Next Steps

- [Downstream APIs Configuration](./06-downstream-apis.md) - Learn how to configure routes and downstream services
- [Deploy Locally](./05-deploy-locally.md) - Set up a complete development environment
- [Quick Start](./03-quick-start.md) - Get started with a minimal configuration

## Troubleshooting

**Issue: Configuration file not found**
- Solution: Ensure `SETTINGS_PATH` points to the correct file path

**Issue: TOML parsing errors**
- Solution: Validate TOML syntax using an online validator or `tomlfmt`

**Issue: Vault connection fails**
- Solution: Ensure Vault is running and unsealed, and the token is valid

For more help, visit the [GitHub Issues](https://github.com/LepistaBioinformatics/mycelium/issues) page.
