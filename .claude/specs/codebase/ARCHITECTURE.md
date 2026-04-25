# Mycelium Architecture

**Analyzed: 2026-04-03**

## Overall Pattern: Hexagonal + Clean Architecture

Mycelium implements a **hexagonal architecture** (ports and adapters) combined with **clean architecture** principles:

- **Core domain logic** lives in `core/` (domain entities, use cases, DTOs)
- **Adapters** in `adapters/` (database, cache, email, shared concerns)
- **Ports** in `ports/` (HTTP API, CLI) serving as entry points
- **Lib** in `lib/` (shared utilities, config, OpenAPI, HTTP tools)

### Characteristics
- Strong separation of concerns with trait-based interfaces
- Domain entities define ports (traits); adapters implement them
- Compile-time DI via Shaku for component resolution
- Request/response flow validates domain rules before adapter access

## High-Level Structure

```
mycelium/
├── core/                          # Domain layer (pure business logic)
│   ├── src/domain/               # Domain entities & value objects
│   │   ├── entities/             # Ports (trait interfaces)
│   │   ├── dtos/                 # Data Transfer Objects
│   │   └── utils/                # Domain utilities
│   ├── src/models/               # Configuration models
│   ├── src/use_cases/            # Application orchestration
│   └── src/settings.rs           # Global settings (templates, TOTP domain)
│
├── lib/                           # Shared libraries (reusable across modules)
│   ├── base/                     # Base types (MappedErrors, DTOs, Parent/Children)
│   ├── config/                   # Configuration loading & Vault integration
│   ├── http_tools/              # HTTP utilities (JWT encoding, compression, auth)
│   └── openapi/                 # OpenAPI schema definitions
│
├── adapters/                      # Implementation layer (concrete bindings)
│   ├── diesel/                   # PostgreSQL adapter via Diesel ORM
│   │   ├── sql/                  # DDL migrations
│   │   ├── repositories/         # Repository implementations (CRUD)
│   │   ├── models/               # Diesel model mappings
│   │   └── schema.rs             # Diesel schema definitions
│   ├── kv_db/                    # Redis key-value adapter
│   │   └── repositories/         # KVArtifactRead/Write implementations
│   ├── mem_db/                   # In-memory repository (fast access)
│   ├── notifier/                 # Email notification adapter (Lettre)
│   │   └── repositories/         # Email sending implementations
│   ├── service/                  # Service discovery & management
│   └── shared/                   # Shared adapter utilities (client providers)
│
├── ports/                         # Entry point layer
│   ├── api/                      # HTTP API gateway (Actix-web)
│   │   ├── src/rest/            # REST endpoint handlers
│   │   ├── src/rpc/             # JSON-RPC endpoint handlers
│   │   ├── src/mcp/             # MCP (Model Context Protocol) endpoint
│   │   ├── src/router/          # Gateway routing logic
│   │   ├── src/middleware/      # HTTP middleware (auth, logging, etc.)
│   │   └── src/openapi/         # OpenAPI documentation
│   └── cli/                      # Command-line interface (not deeply explored)
│
└── test/
    └── downstream_service/      # Integration test helper (mock downstream service)
```

## Key Identified Patterns

### 1. Trait-Based Ports (Hexagonal Pattern)

Domain entities define traits as **ports**:
- Located in: `core/src/domain/entities/**/*.rs`
- Example: `UserRegistration` trait (interface in core)
  ```rust
  #[async_trait]
  pub trait UserRegistration: Interface + Send + Sync {
      async fn get_or_create(
          &self,
          user: User,
      ) -> Result<GetOrCreateResponseKind<User>, MappedErrors>;
  }
  ```

### 2. Shaku-Based Dependency Injection

- Framework: **Shaku 0.6** (compile-time DI)
- Pattern:
  ```rust
  #[derive(Component)]
  #[shaku(interface = UserRegistration)]
  pub struct UserRegistrationSqlDbRepository {
      #[shaku(inject)]
      pub db_config: Arc<dyn DbPoolProvider>,
  }
  ```
- Modules use `module! { components = [...] }` for registration
- Examples: `SqlAppModule`, `MemDbAppModule`, `KVAppModule`, `NotifierAppModule`

### 3. Error Handling: MappedErrors

Central custom error type in `mycelium-base`:
- `MappedErrors`: structured error with type, code, message, and stack
- Factory functions: `creation_err()`, `fetching_err()`, `updating_err()`, `use_case_err()`, `dto_err()`
- Error codes via `NativeErrorCodes` enum
- Trait impl: `Display`, serialization to JSON for HTTP responses

### 4. Response Wrapper Types

From `mycelium-base/entities.rs`:
- `CreateResponseKind<T>` → `Created(T)` | `InvalidRepository`
- `FetchResponseKind<T, E>` → `Found(T)` | `NotFound(E)`
- `GetOrCreateResponseKind<T>` → `Created(T)` | `NotCreated(T, reason)`
- `UpdateResponseKind<T>` → `Updated(T)` | `NotUpdated(reason)`
- `DeletionResponseKind` → `Deleted` | `NotDeleted(reason)`

### 5. Parent-Child DTOs

For relationships in `mycelium-base`:
- `Parent<T, K>` → `Id(K)` | `Record(Box<T>)`
- `Children<T, K>` → `Ids(Vec<K>)` | `Records(Vec<T>)`
- Example: Users within an Account use `Parent::Id(account_id)`

### 6. Secret Resolution (Vault Integration)

- Type: `SecretResolver<T>` in `lib/config`
- Async: `.async_get_or_error().await?`
- Used for: JWT secrets, SMTP credentials, OAuth2 configs
- Storage: Lazy-static cached config via `init_vault_config_from_file()`

### 7. Repository Pattern with CRUD

Each entity has separate trait ports:
- `XyzRegistration` (CREATE)
- `XyzFetching` (READ)
- `XyzUpdating` (UPDATE)
- `XyzDeletion` (DELETE)
- Example: `User` has `UserRegistration`, `UserFetching`, `UserUpdating`, `UserDeletion`

## Critical Data Flow Paths

### Path 1: Authentication & Request Routing

```
HTTP Request
    ↓
[Actix-web Server] (ports/api/src/main.rs)
    ↓
[CORS Middleware] → [Logger] → [TraceLogger]
    ↓
[Route Dispatcher] (ports/api/src/router/route_request)
    ↓
1. Match downstream route (MemDbAppModule lookup)
2. Check source reliability (IP whitelist)
3. Check method permission (allowed HTTP methods)
4. Build downstream URL
5. Check security group (extract user context)
6. Inject downstream secret (if needed)
7. Initialize downstream request
8. Stream to downstream service
    ↓
[Gateway Response] → HTTP Response
```

Key file: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/router/mod.rs` (57-174)

### Path 2: User Registration & Authentication

```
REST/RPC Request → begin_registration
    ↓
[Use Case] (core/src/use_cases/role_scoped/beginner/user/*)
    ↓
1. Validate email/user data
2. Call UserRegistration::get_or_create (repository port)
3. Inject identity provider (internal/OAuth2)
4. Return user with initial role
    ↓
[Store in DB] (UserRegistrationSqlDbRepository)
    ├── INSERT user row (UUID primary key)
    ├── INSERT identity_provider row
    └── Return GetOrCreateResponseKind
    ↓
[Send notification] (LocalMessageWrite port)
    ├── Store to DB (message table)
    └── Optionally send email (RemoteMessageWrite)
```

Key files:
- `core/src/domain/entities/user/user_registration.rs` (trait)
- `adapters/diesel/src/repositories/user/user_registration.rs` (implementation)
- `core/src/use_cases/role_scoped/beginner/user/` (orchestration)

### Path 3: MCP (Model Context Protocol) Tool Invocation

```
HTTP POST /mcp
    ↓
[MCP Endpoint] (ports/api/src/mcp/endpoints.rs)
    ↓
Parse JSON-RPC request
    ↓
Dispatch by method:
├── "initialize" → handle_initialize
├── "tools/list" → handle_list_tools (from ServiceOpenApiSchema registry)
└── "tools/call" → handle_call_tool
    ↓
[Tool Handler] (ports/api/src/mcp/handlers/)
    ↓
Build HTTP call to downstream service
    ├── Extract tool parameters
    ├── Inject authentication
    └── Stream response back as JSON-RPC
```

Key file: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/mcp/endpoints.rs` (1-89)

## Module Boundaries

### Core Module Exports
- `domain::entities` — trait ports (interfaces for adapters)
- `domain::dtos` — data types (value objects, serializable types)
- `use_cases` — application orchestration (depends on port traits)
- `models::AccountLifeCycle`, `settings` — configuration

### Adapter Boundaries
Each adapter is a standalone crate with its own Cargo.toml:
- **diesel**: PostgreSQL bindings only; uses `#[shaku(interface = Trait)]` to satisfy core ports
- **kv_db**: Redis-specific; implements `KVArtifactRead/Write` ports
- **mem_db**: In-memory storage; implements same ports as diesel (test friendly)
- **notifier**: Email only; implements `LocalMessageWrite` and `RemoteMessageWrite`
- **service**: Gateway service definitions and routing rules
- **shared**: Common client provider utilities (`SharedClientImpl`, credential management)

### Lib Module Exports
- **base**: `MappedErrors`, `Parent/Children`, response kinds
- **config**: `SecretResolver`, `VaultConfig`, `init_vault_config_from_file()`
- **http_tools**: JWT encoding, compression, auth helpers, external provider configs
- **openapi**: `ApiDoc`, schema utilities

### Port Exports
- **api**: Actix-web server, all endpoints (REST, RPC, MCP)
- **cli**: Command-line tools (under-explored; appears minimal)

## Dependency Direction

Follows **dependency inversion** principle:

```
             core/
       (domain, use_cases)
           ↑↑↑↑↑↑↑
          /         \
    lib/           adapters/
  (shared)    (diesel, redis, mem, email)
```

- Core has **no dependencies** on adapters
- Adapters depend on core (implement its trait ports)
- Lib provides utilities to both
- Ports (API, CLI) depend on core + adapters + lib

## Deployment Considerations

### Docker Staging
1. **Build stage**: `rust:latest` compiler
2. **Production stage**: `rust:latest` with binary installed
3. Runtime needs:
   - `TEMPLATES_DIR` environment variable
   - PostgreSQL connection string
   - Redis URL (optional, for caching)
   - Vault endpoint (for secrets)
   - OTEL collector endpoint (for observability)

### Configuration Injection
- Via `myc_config::load_config_from_file()` (TOML)
- Via environment variables (`SecretResolver`)
- Via Docker build args (`VERSION`, `SERVICE_PORT`, `TEMPLATES_DIR`)

## Observability Architecture

### Tracing Strategy
- Framework: `tracing` 0.1 with `tracing-subscriber`
- Destinations:
  - **Console/File**: via `tracing-appender`
  - **OpenTelemetry Collector**: via `opentelemetry-otlp` (gRPC or HTTP)
- Instrumentation:
  - `#[tracing::instrument]` decorates all async functions
  - Span fields captured: request ID, method, protocol, response status/duration
  - Example: `route_request` span in `router/mod.rs`

### Logging Format
- JSON format option (`tracing-subscriber::fmt` with `json` feature)
- Environment filtering: `EnvFilter` from `RUST_LOG`
- File appender with automatic rotation

## Feature Flags

### Known Features
- **rhai**: Optional scripting engine (mentioned in Dockerfile and Cargo.toml)
  - Used for: Likely dynamic rule evaluation or callback engines
  - File: `/mnt/external/thirdparty-projects/mycelium/ports/api/src/callback_engines/`

---

## Diagram: High-Level Request Flow

```
┌──────────────────────────────────────────────────────────────┐
│                    HTTP Client                                │
└─────────────────────────┬──────────────────────────────────────┘
                          │ HTTP Request
                          ↓
┌──────────────────────────────────────────────────────────────┐
│                  Actix-web Server                             │
│  (ports/api/src/main.rs - App setup)                          │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ↓
┌──────────────────────────────────────────────────────────────┐
│              Middleware Stack                                 │
│  ├─ CORS (actix-cors)                                        │
│  ├─ Logger (tracing-actix-web)                               │
│  ├─ TraceLogger (tracing)                                    │
│  └─ Auth (actix-web-httpauth)                                │
└─────────────────────────┬──────────────────────────────────────┘
                          │
              ┌───────────┼───────────┐
              ↓           ↓           ↓
        REST /api    RPC /rpc    MCP /mcp
        Endpoints    Handlers    Endpoints
              │           │           │
              └───────────┼───────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│         Gateway Router (router/mod.rs)                        │
│  1. Match route from MemDb                                   │
│  2. Security checks (IP, method)                             │
│  3. Build downstream URL                                     │
│  4. Inject secrets/auth                                      │
│  5. Stream to downstream service                             │
└─────────────────────────┬──────────────────────────────────────┘
                          │
              ┌───────────┼────────────────────────┐
              ↓           ↓                        ↓
        PostgreSQL    Redis             Downstream
        (Diesel)      (KV store)        Services
              │           │                   │
              └─────┬─────┴─────────────────────┘
                    │
                    ↓
        ┌──────────────────────┐
        │   HTTP Response      │
        └──────────────────────┘
```

