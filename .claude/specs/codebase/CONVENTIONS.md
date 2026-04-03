# Mycelium Code Conventions

**Analyzed: 2026-04-03**

## File & Module Organization

### Module Naming
- **snake_case** for all module and file names
- **Collective modules** use `mod.rs` pattern:
  - Example: `core/src/domain/entities/user/mod.rs` re-exports submodules
  - Pattern:
    ```rust
    mod user_deletion;
    mod user_fetching;
    mod user_registration;
    mod user_updating;
    
    pub use user_deletion::*;
    pub use user_fetching::*;
    pub use user_registration::*;
    pub use user_updating::*;
    ```

### File Organization by Function
Separation of responsibility into single-trait files:
- `user_registration.rs` — only `UserRegistration` trait/impl
- `user_fetching.rs` — only `UserFetching` trait/impl
- `user_deletion.rs` — only `UserDeletion` trait/impl
- `user_updating.rs` — only `UserUpdating` trait/impl

Observed in:
- `core/src/domain/entities/user/` (trait definitions)
- `adapters/diesel/src/repositories/user/` (implementations)
- `adapters/notifier/src/repositories/` (email implementations)

### Directory Hierarchy
- **entities/** — domain trait ports
- **dtos/** — data transfer objects (serializable types)
- **use_cases/** — application orchestration logic
- **repositories/** — concrete implementations (per adapter)
- **models/** — data models (database, configuration)
- **schema/** — Diesel ORM schema definitions

## Trait Naming Conventions

### Port Trait Naming
Format: `{Entity}{Verb}` (action-based)
- `UserRegistration` — CREATE operation
- `UserFetching` — READ operations
- `UserUpdating` — UPDATE operations
- `UserDeletion` — DELETE operations
- `KVArtifactRead` — Redis read access
- `KVArtifactWrite` — Redis write access
- `RemoteMessageWrite` — Email sending
- `LocalMessageWrite` — Database message storage

### Trait Definition Pattern
Located in `core/src/domain/entities/`:
```rust
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors>;
}
```

### Implementation Naming
Format: `{Trait}SqlDbRepository` or `{Trait}{Technology}Repository`
- `UserRegistrationSqlDbRepository` — Diesel/PostgreSQL
- `KVArtifactReadRepository` — Redis
- `RemoteMessageSendingRepository` — Lettre/SMTP
- `UserMemDbRepository` — In-memory (if applicable)

## Error Handling

### Custom Error Type: MappedErrors
Located: `mycelium-base/utils/errors/`

Structure:
```rust
pub struct MappedErrors {
    pub error_type: ErrorType,
    pub error_codes: ErrorCodes,
    pub message: String,
    pub stack: Vec<String>,
    pub expected: bool,  // Is this an expected/handled error?
}
```

### Error Type Enum
Categories:
- `UndefinedError` (default)
- `CreationError`, `UpdatingError`, `UpdatingManyError` (CRUD)
- `FetchingError`, `DeletionError`
- `UseCaseError`, `ExecutionError`
- `InvalidRepositoryError`, `InvalidArgumentError`
- `DataTransferLayerError`
- `GeneralError(String)`

### Error Factory Functions
From `mycelium-base/utils/errors/factories.rs`:
```rust
creation_err("message")        // → CreationError
fetching_err("message")        // → FetchingError
updating_err("message")        // → UpdatingError
deletion_err("message")        // → DeletionError
use_case_err("message")        // → UseCaseError
execution_err("message")       // → ExecutionError
dto_err("message")             // → DataTransferLayerError
```

### Error Code Assignment
Via `.with_code()` method:
```rust
creation_err("Provider is required to create a user")
    .with_code(NativeErrorCodes::MYC00002)
```

Native codes: `NativeErrorCodes` enum in `core/src/domain/dtos/native_error_codes.rs`

### Expected Errors
Mark handled errors with `.with_exp_true()`:
```rust
use_case_err(format!("User already has TOTP enabled: {}", email.email()))
    .with_code(NativeErrorCodes::MYC00021)
    .with_exp_true()
    .as_error()
```

## Comments & Documentation

### Doc Comments
Rust doc comment style with examples:
```rust
/// This enumerator are used to standardize errors codes dispatched during the
/// `MappedErrors` struct usage.
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum ErrorType {
    /// This error type is used when the error type is not defined. This is the
    /// default value for the `ErrorType` enum.
    ///
    /// Related: Undefined
    UndefinedError,
```

### Inline Comments
Markdown-style section headers in code:
```rust
// ? -----------------------------------------------------------------------
// ? Initialize route span
// ? -----------------------------------------------------------------------

let span = tracing::Span::current();
```

### TODO/FIXME
Found in:
- `ports/api/src/router/mod.rs` (line 53-55): X-Forwarded-For incomplete
- `core/src/domain/dtos/http_secret.rs` (line 66): Client certificate auth not implemented
- `ports/api/src/rest/manager/guest_role_endpoints.rs` (line 27): Not yet implemented

## Import Organization

### Standard Pattern
1. **Relative/crate imports** (no dependencies on impl)
2. **Async-trait and framework imports**
3. **Core domain imports** (entities, DTOs)
4. **Adapter/library imports** (shaku, traits)
5. **Standard library imports**

Example from `adapters/diesel/src/repositories/user/user_registration.rs`:
```rust
use crate::{                          // Adapter-local
    models::{...},
    schema::{...},
};

use async_trait::async_trait;         // Framework
use chrono::Local;
use diesel::prelude::*;               // ORM

use myc_core::domain::{               // Core domain
    dtos::{...},
    entities::UserRegistration,
};

use mycelium_base::{                  // Base utilities
    dtos::Parent,
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};

use shaku::Component;                 // DI
use std::sync::Arc;
use uuid::Uuid;
```

## Visibility & Access Control

### pub/pub(crate) Usage
- **pub** — Exported in public crate API (trait definitions in core)
- **pub(crate)** — Private to crate, used within adapters
- **pub(super)** — Private to parent module (repository submodules)
- **No visibility** — Private to module

Example:
```rust
// core/src/domain/entities/user/mod.rs
pub use user_registration::*;    // Public export
pub use user_fetching::*;

// adapters/diesel/src/repositories/user/mod.rs
pub(super) use user_registration::*;  // Only visible to parent (repositories/)
pub(super) use user_fetching::*;
```

### Repository Pattern
```rust
#[derive(Component)]
#[shaku(interface = UserRegistration)]
pub struct UserRegistrationSqlDbRepository {    // Public (Component needs it)
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,     // Public (Shaku inject)
}

#[async_trait]
impl UserRegistration for UserRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create", skip_all)]
    async fn get_or_create(&self, user: User) -> Result<...> {
        // Implementation
    }
}
```

## Async & Trait Patterns

### async_trait Macro
Used on all trait methods requiring async:
```rust
use async_trait::async_trait;

#[async_trait]
pub trait UserFetching: Interface + Send + Sync {
    async fn get_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User>, MappedErrors>;
}
```

### Instrumentation
Almost all async functions have `#[tracing::instrument]`:
```rust
#[tracing::instrument(name = "get_or_create_user", skip_all)]
async fn get_or_create(
    &self,
    user: User,
) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
    tracing::info!("User already exists: {record:?}");
    tracing::error!("Error on get redis connection: {err}");
}
```

Parameters:
- `name` — operation identifier
- `skip_all` — avoid logging large struct args
- Explicit field capture: `fields(myc.router.req_id = tracing::field::Empty)`

## Struct Derivations

### Common Derives
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]  // Core
#[derive(Component)]                              // Shaku (adapters)
#[derive(ToSchema)]                               // OpenAPI (endpoints)
#[derive(Eq, PartialEq)]                          // Comparisons
```

### Serde Configuration
```rust
#[serde(rename_all = "camelCase")]  // JSON field naming
#[serde(default)]                    // Default values
#[serde(skip_serializing_if = "Option::is_none")]
```

## Response Type Patterns

### Standardized Response Kinds
From `mycelium-base`:
- `CreateResponseKind<T>` → `Created(T)` | `InvalidRepository(msg)`
- `FetchResponseKind<T, E>` → `Found(T)` | `NotFound(E)`
- `GetOrCreateResponseKind<T>` → `Created(T)` | `NotCreated(T, reason)`
- `UpdateResponseKind<T>` → `Updated(T)` | `NotUpdated(reason)`
- `DeletionResponseKind` → `Deleted` | `NotDeleted(reason)`

Example usage:
```rust
async fn get_or_create(...) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
    if let Some(record) = existing_user {
        return Ok(GetOrCreateResponseKind::NotCreated(
            User::new(...),
            "User created if not exists".to_string(),
        ));
    }
    Ok(GetOrCreateResponseKind::Created(new_user))
}
```

## Database & ORM Patterns

### Diesel Pattern
```rust
use diesel::prelude::*;
use crate::models::User as UserModel;
use crate::schema::user as user_model;

// Query building
let existing_user = user_model::table
    .filter(user_model::email.eq(email))
    .select(UserModel::as_select())
    .first::<UserModel>(conn)
    .optional()
    .map_err(|e| creation_err(format!("Failed to check user: {}", e)))?;

// Insert with returning
let created_user = diesel::insert_into(user_model::table)
    .values(new_user)
    .returning(UserModel::as_returning())
    .get_result::<UserModel>(conn)?;

// Transaction
let result = conn.transaction(|conn| {
    // Multiple operations
    Ok(result)
})?;
```

## Function Naming

### Verb-Based
- `get_*` — Fetch single item
- `get_or_create` — Idempotent GET
- `create_*`, `register_*` — INSERT
- `update_*` — UPDATE
- `delete_*` — DELETE
- `check_*` — Validation/assertion
- `initialize_*`, `init_*` — Setup
- `dispatch_*` — Send/execute external action

## Testing Conventions

### Inline Test Modules
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_config_from_file() -> Result<(), MappedErrors> {
        std::env::set_var("ENV_VAR", "env_value");
        
        let config: Config = load_config_from_file(PathBuf::from("tests/config.toml"))?;
        
        assert_eq!(config.name, "Name");
        Ok(())
    }
}
```

### Test Resource Files
Located in `lib/config/tests/config.toml` for test configuration

### Assertion Style
- `assert_eq!()` for equality
- `unwrap_err()` for error assertions
- `?` operator for error propagation

