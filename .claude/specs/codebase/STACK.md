# Mycelium Technology Stack

**Analyzed: 2026-04-03**

## Core Technology Stack

### Language & Runtime
- **Rust** (Edition 2021)
- **Tokio** 1.46 (async runtime, full features)
- **Actix-web** 4.x (HTTP framework)
  - `actix-cors` 0.6.2 (CORS support)
  - `actix-rt` 2.10 (Actix runtime)
  - `actix-web-httpauth` 0.8.0 (authentication middleware)
  - `actix-web-opentelemetry` 0.22.0 (observability integration)

### Database
- **PostgreSQL** (primary data store via Diesel)
- **Diesel** (ORM framework)
  - With custom schema definitions (`adapters/diesel/src/schema`)
  - SQL migrations (`adapters/diesel/sql/up.sql`)
  - Connection pooling via `DieselDbPoolProvider`
  - Version: 8.3.1-beta.5

### Caching & Message Queue
- **Redis** 0.27 (key-value store, pub/sub)
  - Feature: `tokio-comp` (Tokio async support)
  - Used for artifact caching and message queuing
  - Accessed via `redis::Commands` and `FromRedisValue` trait

### In-Memory Database
- **Memory-based repository** (`mycelium-memory-db` adapter)
  - Used for local/fast-access data without persistence
  - Implements same interface pattern as SQL adapter

## Authentication & Security

### JWT/Token Handling
- **jsonwebtoken** 10 (with `aws_lc_rs` feature)
- **jwt** 0.16.0
- **pasetors** 0.6 (authenticated encryption alternative to JWT)
- **argon2** 0.5 (password hashing)
- **ring** 0.17 (cryptography primitives)
- **hmac** 0.12 (HMAC authentication)
- **sha2** 0.10 (SHA-2 hashing)
- **base32** 0.4 (base32 encoding for TOTP secrets)
- **totp-rs** 5.0 (Time-based One-Time Password with QR code generation)
- **base64** 0.22 (base64 encoding)
- **hex** 0.4.3 (hex encoding)

### OAuth2 & External Identity
- **oauth2** 4.4 (OAuth2 client framework)
- **External provider support** (Google, Microsoft, Auth0 via OIDC)
  - Configured via `ExternalProviderConfig`
  - JWKS URI validation
  - Discovery URL support

### SSL/TLS
- **openssl** >= 0.10.70 (SSL/TLS support)
- **openssl** for cert parsing in `Dockerfile`

### Vault Integration
- **Custom SecretResolver** (`myc_config::secret_resolver::SecretResolver`)
  - Async secret resolution from Vault
  - Used for JWT secrets, SMTP credentials, OAuth2 secrets
  - Located: `lib/config/src/domain/dtos/secret_resolver.rs`

## HTTP & API

### HTTP Client
- **awc** 3 (async HTTP client, part of Actix ecosystem)
  - Features: `openssl` (SSL/TLS)
  - Used for downstream service calls
- **reqwest** 0.11 (HTTP client with native-tls)
  - Features: `json`, `native-tls` (avoiding unmaintained rustls-pemfile)

### API Documentation & OpenAPI
- **utoipa** 5 (OpenAPI schema generation)
  - Features: `actix_extras`, `chrono`, `debug`, `openapi_extensions`, `preserve_order`, `uuid`
  - `utoipa-swagger-ui` 9 (Swagger UI support)
  - `utoipa-redoc` 5 (ReDoc support)
- **schemars** 1.2.1 (JSON Schema generation)

### Serialization
- **serde** 1.0 (serialization framework)
- **serde_json** 1.0 (JSON support)
- **toml** 0.9 (TOML configuration parsing)

## Observability & Monitoring

### Tracing & Logging
- **tracing** 0.1 (structured tracing)
- **tracing-subscriber** 0.3.19 (tracing subscriber with features: `json`, `env-filter`, `tracing-serde`, `registry`)
- **tracing-appender** 0.2 (file-based logging)
- **tracing-actix-web** 0.7 (Actix-web integration)
- **tracing-opentelemetry** 0.32 (OpenTelemetry bridge)
- **env_logger** 0.10 (environment-based log filtering)

### OpenTelemetry (OTEL)
- **opentelemetry** 0.31 (base package)
  - Features: `trace`, `metrics`, `logs`
- **opentelemetry_sdk** 0.31 (OpenTelemetry SDK)
  - Feature: `rt-tokio` (Tokio runtime support)
- **opentelemetry-otlp** 0.31 (OTLP exporter)
  - Features: `reqwest-client`, `reqwest-rustls`, `http-proto`, `tls`, `grpc-tonic`, `trace`, `metrics`, `logs`
- **tonic** 0 (gRPC support for OTLP/gRPC)
  - Features: `tls`, `tls-roots`

## Email & Notifications

### Email Sending
- **lettre** 0.11 (SMTP email client)
  - Used via `RemoteMessageSendingRepository`
  - Configured with SMTP credentials from Vault
  - Supports HTML content via `ContentType::TEXT_HTML`

## Dependency Injection

### DI Framework
- **shaku** 0.6 (compile-time dependency injection)
  - `#[derive(Component)]` macro for repository implementations
  - `#[shaku(interface = Trait)]` for trait binding
  - `#[shaku(inject)]` for dependency injection
  - Module pattern: `module! { pub XyzModule { components = [...], providers = [] } }`
  - Used in all adapter layers and API configuration

## Utilities & Helpers

### Data & Time
- **chrono** 0.4 (date/time handling with serde support)
- **time** 0.3.47 (alternative time library)
- **uuid** 1.1 (UUID generation with features: `v3`, `v4`, `v7`, `serde`, `fast-rng`)

### Async & Concurrency
- **async-trait** 0.1 (async trait support)
- **futures** 0.3 (future combinators)
- **futures-util** 0.3 (utility functions for futures)
- **tokio** 1.46 (async runtime)

### String Processing
- **regex** 1 (regular expressions)
- **slugify** 0.1.0 (URL-friendly slugs)
- **wildmatch** 2.1 (wildcard pattern matching)
- **url** 2.2 (URL parsing and manipulation)

### Serialization & Encoding
- **derive_more** 0.99 (derive macro extensions)
- **lazy_static** 1.4 (static initialization)
- **zstd** 0.13 (Zstandard compression)
- **zip** 2.4.2 (ZIP archive handling)

### Random Number Generation
- **rand** 0.8 (RNG for cryptographic operations)

## Testing Frameworks

### Testing
- **test-log** 0.2.8 (logging in tests)
- **mockall** 0.11.4 (mocking framework)
- Inline `#[cfg(test)]` modules in source files
- Test configuration files: `lib/config/tests/config.toml`

## Build & Deployment

### Build Tools
- **Cargo** (Rust package manager)
- **Workspace resolver** 2 (unified dependency resolution)
- **Release configuration** in `cliff.toml` (Cargo-cliff)

### Docker
- Multi-stage Docker builds:
  - **Builder stage**: `rust:latest` (full build environment)
  - **Production stage**: `rust:latest` (runtime environment)
  - Features: optional `rhai` feature flag
  - Entry point: `myc-api` binary
  - Template files required at runtime

### Development Tools
- **Dockerfile.dev** (development image)
- **Dockerfile.test** (test runner image)
- **docker-compose.yaml** (local orchestration)
- **docker-compose.common.yaml** (shared configuration)

### Scripts
- **scripts/publish-all.sh** (package publishing)
- **scripts/run-test-servers.sh** (test service runner)
- **scripts/vault/vault-init.sh** (Vault initialization)

## Code Quality & Security

### Code Quality
- **cargo fmt** for code formatting
- Checked in CI pipeline

### Security Dependencies
- **RUSTSEC-2025-0134**: Force use of native-tls over rustls-pemfile (unmaintained)
- **RUSTSEC-2026-0007**: bytes >= 1.11.1 (integer overflow fix in BytesMut::reserve)
- **RUSTSEC-2026-0009**: time >= 0.3.47 (DoS via stack exhaustion)
- **zip** 2.4.2 (transitive security fix)

### Linting & CI
- GitHub Actions CI:
  - Build check
  - Test execution (`cargo test --workspace --all`)
  - Format check (`cargo fmt --all -- --check`)
  - Clippy linting available (observed in CI patterns)
  - Security audit monitoring (security.yml)

## Version Information
- **Package Version**: 8.3.1-beta.5
- **Edition**: 2021
- **Authors**: Samuel Galvão Elias <sgelias@outlook.com>
- **License**: Apache-2.0
- **Repository**: https://github.com/sgelias/mycelium
