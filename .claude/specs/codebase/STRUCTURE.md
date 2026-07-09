# Mycelium Project Structure

**Analyzed: 2026-04-03**

## Directory Tree (3 Levels Deep)

```
mycelium/
├── adapters/
│   ├── diesel/              PostgreSQL ORM adapter (Diesel)
│   │   ├── sql/             DDL migrations (CREATE TABLE, etc.)
│   │   ├── src/
│   │   │   ├── repositories/  CRUD implementations per entity
│   │   │   ├── models/       Diesel model structs (DB row mappings)
│   │   │   └── schema.rs     Diesel schema definitions
│   │   ├── Cargo.toml
│   │   └── diesel.toml      Diesel CLI configuration
│   ├── kv_db/               Redis adapter
│   │   ├── src/
│   │   │   └── repositories/  KVArtifactRead/Write impls
│   │   └── Cargo.toml
│   ├── mem_db/              In-memory database adapter
│   │   ├── src/
│   │   │   └── repositories/  RAM-based CRUD impls
│   │   └── Cargo.toml
│   ├── notifier/            Email & notification adapter
│   │   ├── src/
│   │   │   ├── repositories/  Lettre SMTP email sending
│   │   │   ├── models/       Email config models
│   │   │   └── executor/     Message dispatch logic
│   │   └── Cargo.toml
│   ├── service/             Service discovery & routing
│   │   ├── src/
│   │   │   └── repositories/  Service definitions
│   │   └── Cargo.toml
│   ├── shared/              Shared adapter utilities
│   │   ├── src/
│   │   │   └── models/      ClientProvider, pool management
│   │   └── Cargo.toml
│   └── Cargo.toml           Workspace member (aggregates dependencies)
├── core/                    Domain layer (pure business logic)
│   ├── src/
│   │   ├── domain/          Business entities and ports
│   │   │   ├── entities/    Trait definitions (ports)
│   │   │   │   ├── user/
│   │   │   │   ├── account/
│   │   │   │   ├── tenant/
│   │   │   │   ├── token/
│   │   │   │   ├── session_token/
│   │   │   │   ├── webhook/
│   │   │   │   ├── kv_artifact/
│   │   │   │   ├── message/
│   │   │   │   ├── service/
│   │   │   │   ├── route/
│   │   │   │   ├── health_check_info/
│   │   │   │   └── mod.rs
│   │   │   ├── dtos/        Data transfer objects
│   │   │   │   ├── user.rs
│   │   │   │   ├── account/
│   │   │   │   ├── tenant/
│   │   │   │   ├── token/
│   │   │   │   ├── message.rs
│   │   │   │   ├── webhook/
│   │   │   │   ├── native_error_codes.rs
│   │   │   │   └── ... (30+ more DTOs)
│   │   │   └── utils/       Domain utilities (UUID conversion, etc.)
│   │   ├── use_cases/       Application orchestration
│   │   │   ├── role_scoped/
│   │   │   │   ├── beginner/
│   │   │   │   ├── account_manager/
│   │   │   │   ├── subscriptions_manager/
│   │   │   │   └── tenant_manager/
│   │   │   ├── gateway/     Gateway-specific use cases
│   │   │   ├── support/     Shared use case utilities
│   │   │   └── mod.rs
│   │   ├── models/          Core configuration models
│   │   │   ├── account_life_cycle.rs
│   │   │   └── config.rs
│   │   ├── lib.rs           Module re-exports
│   │   ├── settings.rs      Global settings (TOTP_DOMAIN, etc.)
│   │   └── mod.rs
│   ├── Cargo.toml
│   ├── CHANGELOG.md
│   └── README.md
├── lib/                     Shared libraries (multi-crate reuse)
│   ├── base/                Base types & utilities
│   │   ├── src/
│   │   │   ├── dtos/        Parent, Children, generic types
│   │   │   ├── entities/    Response kinds (Create, Fetch, etc.)
│   │   │   ├── utils/
│   │   │   │   └── errors/  MappedErrors, ErrorType, factories
│   │   │   └── lib.rs
│   │   ├── Cargo.toml
│   │   └── CHANGELOG.md
│   ├── config/              Configuration management
│   │   ├── src/
│   │   │   ├── domain/
│   │   │   │   └── dtos/    SecretResolver, VaultConfig
│   │   │   ├── models/      Configuration structs
│   │   │   ├── use_cases/   Config loading logic
│   │   │   ├── settings.rs  Lazy-static config cache
│   │   │   └── lib.rs
│   │   ├── tests/           Test config files (config.toml)
│   │   ├── Cargo.toml
│   │   └── CHANGELOG.md
│   ├── http_tools/          HTTP utilities
│   │   ├── src/
│   │   │   ├── functions/   JWT encoding, compression
│   │   │   ├── models/      ExternalProviderConfig, auth
│   │   │   ├── settings/    HTTP constants
│   │   │   ├── utils/       Response helpers
│   │   │   └── lib.rs
│   │   ├── test/
│   │   ├── Cargo.toml
│   │   └── CHANGELOG.md
│   ├── openapi/             OpenAPI/Swagger schema
│   │   ├── src/
│   │   │   └── dtos/        OpenAPI schema definitions
│   │   ├── Cargo.toml
│   │   └── CHANGELOG.md
│   └── README.md
├── ports/                   Entry points (HTTP API, CLI)
│   ├── api/                 HTTP API Gateway (Actix-web)
│   │   ├── src/
│   │   │   ├── rest/        REST endpoints
│   │   │   │   ├── index/   /health, /info endpoints
│   │   │   │   ├── manager/ Account/tenant management
│   │   │   │   ├── role_scoped/  User role-based endpoints
│   │   │   │   ├── service/ Service tools
│   │   │   │   ├── openid/  OpenID/.well-known endpoints
│   │   │   │   ├── staff/   Admin endpoints
│   │   │   │   └── shared.rs Shared utilities
│   │   │   ├── rpc/         JSON-RPC endpoints
│   │   │   │   ├── dispatchers/  Method implementations
│   │   │   │   ├── openrpc/  OpenRPC schema
│   │   │   │   ├── types/   JSON-RPC types
│   │   │   │   └── mod.rs
│   │   │   ├── mcp/         MCP (Model Context Protocol)
│   │   │   │   ├── endpoints.rs  /mcp handler
│   │   │   │   ├── handlers/    Tool invocation logic
│   │   │   │   └── dtos/    MCP message types
│   │   │   ├── router/      Gateway routing logic
│   │   │   │   ├── mod.rs   Main routing orchestrator
│   │   │   │   ├── match_downstream_route_from_request.rs
│   │   │   │   ├── check_source_reliability.rs
│   │   │   │   ├── check_security_group.rs
│   │   │   │   ├── stream_request_to_downstream.rs
│   │   │   │   └── ... (other routing steps)
│   │   │   ├── middleware/  HTTP middleware
│   │   │   ├── models/      API configuration
│   │   │   ├── openapi/     OpenAPI documentation
│   │   │   ├── callback_engines/  Callback/rule evaluation
│   │   │   ├── modifiers/   Request/response modifiers
│   │   │   ├── dtos/        API DTOs
│   │   │   ├── otel.rs      OpenTelemetry setup
│   │   │   ├── main.rs      Server setup & routing
│   │   │   ├── settings.rs  API settings constants
│   │   │   └── mod.rs
│   │   ├── Cargo.toml
│   │   └── CHANGELOG.md
│   └── cli/                 Command-line interface
│       ├── src/
│       │   ├── cmds/        Command implementations
│       │   ├── main.rs
│       │   └── mod.rs
│       ├── Cargo.toml
│       └── CHANGELOG.md
├── test/
│   └── downstream_service/  Integration test helper
│       ├── src/
│       │   ├── endpoints.rs  Mock endpoint definitions
│       │   ├── openapi.rs    Mock OpenAPI schema
│       │   └── main.rs       Mock server startup
│       ├── Cargo.toml
│       └── CHANGELOG.md
├── otel/                    OpenTelemetry configuration
│   ├── otel-collector-config.dev.yaml  Collector config
│   └── prometheus.dev.yml   Metrics config
├── postgres/                PostgreSQL setup & migrations
│   ├── sql/
│   │   └── up.sql           DDL for all tables
│   └── volume/              Data directory (runtime)
├── docs/                    Documentation
│   ├── book/               Mdbook documentation
│   │   ├── src/
│   │   ├── theme/
│   │   └── book.toml
│   ├── assets/             Logos, diagrams
│   ├── deps/               Dependency graphs
│   └── draw.io/            Architecture diagrams
├── scripts/                Build & utility scripts
│   ├── publish-all.sh      Publish all crates
│   ├── run-test-servers.sh  Test server runner
│   └── vault/              Vault initialization
├── .github/
│   └── workflows/          CI/CD configuration
│       ├── ci.yml          Build/test/format checks
│       ├── security.yml    Security scanning
│       ├── deploy-docs.yml Documentation deployment
│       └── claude-pr-review.yml.disabled
├── .devcontainer/          Dev container setup
├── templates/              Email/notification templates
├── Cargo.toml              Workspace root (members, version)
├── Cargo.lock              Locked dependency versions
├── docker-compose.yaml     Local services (DB, Redis, etc.)
├── docker-compose.common.yaml  Shared compose config
├── Dockerfile              Production multi-stage build
├── Dockerfile.dev          Development image
├── Dockerfile.test         Test runner image
├── .env                    Environment variables
├── .env.example            Example env template
├── LICENSE                 Apache 2.0
├── README.md               Project overview
├── CONTRIBUTING.md         Contribution guidelines
├── CODE_OF_CONDUCT.md      Community standards
├── cliff.toml              Changelog generation config
└── .gitignore              Git ignore rules
```

## Purpose of Major Directories

### `core/` — Domain Layer
**Purpose**: Pure business logic independent of frameworks or adapters
- **entities/** → Trait ports (interfaces adapters must implement)
- **dtos/** → Serializable data structures (value objects)
- **use_cases/** → Application orchestration (depends on port traits)
- **models/** → Configuration and lifecycle constants
- **settings.rs** → Global settings (templates directory, TOTP issuer)

**Key Files**:
- `core/src/domain/dtos/user.rs` — User DTO
- `core/src/domain/entities/user/user_registration.rs` — UserRegistration trait
- `core/src/domain/entities/native_error_codes.rs` — Error code enum
- `core/src/use_cases/role_scoped/beginner/user/` — User registration flow

### `adapters/` — Implementation Layer
**Purpose**: Concrete implementations of domain ports
- **diesel/** → PostgreSQL via Diesel ORM
- **kv_db/** → Redis key-value store
- **mem_db/** → In-memory (testing, fast access)
- **notifier/** → Email via Lettre
- **service/** → Service definitions and routing rules
- **shared/** → Common client provider infrastructure

**Key Files**:
- `adapters/diesel_postgres/src/repositories/user/user_registration.rs` — DB implementation
- `adapters/diesel_postgres/sql/up.sql` — DDL migrations (CREATE TABLE)
- `adapters/notifier/src/repositories/remote_message_sending.rs` — SMTP sending

### `lib/` — Shared Utilities
**Purpose**: Reusable across core, adapters, and ports
- **base/** → MappedErrors, response kinds, Parent/Children
- **config/** → SecretResolver, Vault integration, config loading
- **http_tools/** → JWT encoding, compression, auth helpers
- **openapi/** → OpenAPI schema definitions

**Key Files**:
- `lib/base/src/utils/errors/base.rs` — MappedErrors struct and ErrorType enum
- `lib/config/src/domain/dtos/secret_resolver.rs` — Async secret resolution
- `lib/http_tools/src/models/external_providers_config.rs` — OAuth2 config

### `ports/api/` — HTTP Entry Point
**Purpose**: Actix-web server exposing REST, RPC, MCP endpoints
- **rest/** → REST endpoints (traditional HTTP)
- **rpc/** → JSON-RPC 2.0 endpoints
- **mcp/** → Model Context Protocol (Claude AI integration)
- **router/** → Gateway request routing logic
- **middleware/** → Authentication, logging, CORS
- **openapi/** → OpenAPI/Swagger documentation

**Key Files**:
- `ports/api/src/main.rs` — Server setup, module initialization, middleware chain
- `ports/api/src/router/mod.rs` — Request routing orchestration (lines 57-174)
- `ports/api/src/mcp/endpoints.rs` — /mcp JSON-RPC endpoint handler
- `ports/api/src/otel.rs` — OpenTelemetry initialization

### `test/downstream_service/` — Integration Test Helper
**Purpose**: Mock downstream service for testing gateway routing
- Simulates a real downstream API service
- Helps test request forwarding and response streaming
- Runnable via `scripts/run-test-servers.sh`

## Where Things Live

### Authentication & JWT
- Core trait: `core/src/domain/entities/` (multiple, mixed with other entities)
- JWT encoding: `lib/http_tools/src/functions/encode_jwt.rs`
- Config: `lib/http_tools/src/models/external_providers_config.rs` (OAuth2/OIDC)
- Secret resolution: `lib/config/src/domain/dtos/secret_resolver.rs`
- Middleware: `ports/api/src/middleware/get_email_or_provider_from_request.rs`

### Routing & Gateway
- Main router: `ports/api/src/router/mod.rs`
- Route matching: `ports/api/src/router/match_downstream_route_from_request.rs`
- Security checks: `ports/api/src/router/check_source_reliability.rs`, `check_security_group.rs`
- Downstream forwarding: `ports/api/src/router/stream_request_to_downstream.rs`

### Database & Migrations
- DDL: `adapters/diesel_postgres/sql/up.sql` (single file with all CREATE TABLE statements)
- Schema: `adapters/diesel_postgres/src/schema.rs` (Diesel schema definitions)
- Models: `adapters/diesel_postgres/src/models/` (Diesel row structs)
- Repositories: `adapters/diesel_postgres/src/repositories/` (CRUD per entity)

### Configuration & Secrets
- Loading: `lib/config/src/use_cases/load_config_from_file.rs`
- Vault: `lib/config/src/domain/dtos/secret_resolver.rs`
- Initialization: `lib/config/src/settings.rs`
- TOML parsing: All config leverages `serde` + `toml` crate

### Testing
- Inline tests: `#[cfg(test)]` modules within source files
  - Example: `lib/config/src/use_cases/load_config_from_file.rs` (lines 28-58)
- Test resources: `lib/config/tests/config.toml`
- Integration tests: `test/downstream_service/`
- Mocking: `mockall` crate for trait mocking

### Observability
- Tracing setup: `ports/api/src/otel.rs` (lines 1-200+)
- Logging levels: `RUST_LOG` environment variable
- Log output: File or STDERR via `tracing-appender`
- OTEL collector: `otel/otel-collector-config.dev.yaml`
- Metrics: `otel/prometheus.dev.yml`

### Error Handling
- Error types: `mycelium-base` crate (`lib/base/src/utils/errors/`)
  - `MappedErrors` struct definition
  - `ErrorType` enum (creation, fetching, deletion, etc.)
  - Factory functions (creation_err, fetching_err, etc.)
- Error codes: `core/src/domain/dtos/native_error_codes.rs`

### Email & Notifications
- SMTP client: `adapters/notifier/src/` (Lettre integration)
- Config: `adapters/notifier/src/models/config.rs`
- Sending: `adapters/notifier/src/repositories/remote_message_sending.rs`
- Templates: `templates/` directory (referenced by environment variable)

### MCP (Model Context Protocol)
- Endpoint: `ports/api/src/mcp/endpoints.rs` (JSON-RPC router)
- Handlers: `ports/api/src/mcp/handlers/` (tool list, call logic)
- Tool registry: `ports/api/src/openapi_processor.rs` (ServiceOpenApiSchema)
- DTOs: `ports/api/src/mcp/dtos/` (JSON-RPC message types)

## Build Artifacts

### Compiled Binaries
- **myc-api** — HTTP API server (from `ports/api`)
- **myc-cli** — Command-line tool (from `ports/cli`)
- **myc-api-test-svc** — Test service (from `test/downstream_service`)

### Docker Images
- **Production**: Multi-stage from `Dockerfile`
  - Stage 1: Build from `rust:latest`
  - Stage 2: Runtime from `rust:latest`
- **Development**: `Dockerfile.dev` (with hot-reload setup)
- **Testing**: `Dockerfile.test` (cargo test runner)

### Configuration Files
- **docker-compose.yaml** — Local orchestration (PostgreSQL, Redis, etc.)
- **docker-compose.common.yaml** — Shared services
- **.env** — Runtime environment (credentials, URLs)

