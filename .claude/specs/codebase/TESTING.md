# Mycelium Testing Guide

**Analyzed: 2026-04-03**

## Test Frameworks & Tools

### Test Execution Framework
- **Tokio** 1.46 with `#[tokio::test]` macro for async tests
- **Cargo test** as the test runner
- **test-log** 0.2.8 for capturing logs in tests
- **mockall** 0.11.4 for trait mocking

### Test Organization
Tests are organized using **inline `#[cfg(test)]` modules** within source files, not in separate test files.

Pattern:
```rust
// In the same file as the implementation

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_something() -> Result<(), MappedErrors> {
        // test implementation
        Ok(())
    }
}
```

## Test Locations

### Unit Tests
Located inline with source code:

**Examples from codebase**:
- `lib/config/src/use_cases/load_config_from_file.rs` (lines 28-58)
  - Tests TOML config loading with SecretResolver
  - Environment variable injection
  - Assertion: `assert_eq!(config.var_with_env.get_or_error()?, "env_value")`

- `lib/base/src/utils/errors/base.rs`
  - ErrorType enum serialization/deserialization
  - Display trait implementations

- `lib/base/src/utils/errors/factories.rs`
  - Error factory functions
  - Error code assignment

- `lib/http_tools/src/functions/encode_jwt.rs`
  - JWT encoding/decoding tests

- `lib/http_tools/src/functions/decode_and_decompress_profile_from_base64.rs`
  - Base64/compression round-trip tests

- `lib/base/src/dtos/generic_map.rs`
  - Generic data structure serialization

- `core/src/domain/dtos/route.rs`
  - Route matching and URL resolution tests (lines 606-693)
  - Example assertions:
    ```rust
    assert_eq!(result.unwrap(), None);
    let error = result.unwrap_err();
    assert_eq!(result.unwrap(), secret)
    ```

### Integration Tests
- Primary: `test/downstream_service/`
  - Mock downstream HTTP service
  - Used to test gateway request forwarding
  - Runnable via: `scripts/run-test-servers.sh`

- Pattern file: `RUST_LOG=debug SERVICE_PORT=8083 cargo run --package mycelium-api-test-svc --bin myc-api-test-svc`

### Test Configuration Resources
- `lib/config/tests/config.toml` — Test configuration file
  - Used by: `load_config_from_file()` tests
  - Contains sample config with SecretResolver fields

## Testing Patterns Observed

### 1. Async Test Pattern
```rust
#[tokio::test]
async fn test_load_config_from_file() -> Result<(), MappedErrors> {
    // Setup
    std::env::set_var("ENV_VAR", "env_value");
    
    // Action
    let config: Config = load_config_from_file(PathBuf::from("tests/config.toml"))?;
    
    // Assert
    assert_eq!(config.name, "Name");
    assert_eq!(config.age, 99);
    assert_eq!(config.var_with_env.get_or_error()?, "env_value");
    
    Ok(())
}
```

### 2. Result-Based Error Testing
```rust
#[test]
fn test_route_parsing() {
    let result = route.resolve_secret().await;
    assert_eq!(result.unwrap_err(), ErrorCode::InvalidRoute);
}
```

### 3. Mockall Trait Mocking
Framework: **mockall** 0.11.4
- Used to mock domain trait ports in tests
- Allows testing adapters in isolation
- Example traits that are commonly mocked:
  - `UserRegistration`
  - `UserFetching`
  - `DbPoolProvider`

### 4. Error Assertion Pattern
From `core/src/domain/dtos/route.rs` (test section):
```rust
#[test]
fn test_invalid_route_error() {
    let error = result.unwrap_err();
    // Verify error type and code
    assert_eq!(error.error_type, ErrorType::InvalidArgumentError);
}
```

## How to Run Tests

### Run All Tests (Workspace)
```bash
cargo test --workspace --all
```
- Runs all tests in all workspace members
- Includes unit tests and integration tests
- Used in CI pipeline (`.github/workflows/ci.yml`)

### Run Tests for Specific Package
```bash
cargo test -p myc-core
cargo test -p mycelium-base
cargo test -p mycelium-diesel
```

### Run Tests with Log Output
```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Run Single Test
```bash
cargo test test_load_config_from_file -- --exact
```

### Run Integration Tests
```bash
cargo test --test '*'  # explicit test runner
# Or start the test service:
scripts/run-test-servers.sh
```

## CI/CD Testing Pipeline

### GitHub Actions (`.github/workflows/ci.yml`)

**Test Job**:
```yaml
test:
  name: Test
  runs-on: ubuntu-latest
  steps:
    - name: Checkout code
      uses: actions/checkout@v4
    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable
    - name: Cache dependencies
      uses: Swatinem/rust-cache@v2
    - name: Run tests
      run: cargo test --workspace --all
```

### Other CI Checks

**Format Check** (`.github/workflows/ci.yml`):
```bash
cargo fmt --all -- --check
```

**Build Check**:
```bash
cargo build --workspace
```

**Security Scanning** (`.github/workflows/security.yml`):
- Cargo audit for known vulnerabilities
- Dependency scanning

### Pre-Commit Hooks
Not explicitly configured in codebase, but format checking enforced in CI.

## Test Coverage & Gaps

### Well-Tested Areas
1. **Error handling** — Extensive `MappedErrors` tests in `lib/base/`
2. **Configuration loading** — TOML parsing, Vault integration tested
3. **Encoding/compression** — JWT, Base64, compression round-trips verified
4. **Data structures** — DTOs, Parent/Children relationships tested
5. **Route resolution** — URL matching and secret injection logic tested

### Observed Test Coverage Matrix

| Layer | Coverage | Notes |
|-------|----------|-------|
| **Lib (base, config, http_tools)** | High | Inline tests present; error handling well-tested |
| **Core (domain, dtos)** | Medium | Some data structure tests; use cases have limited test coverage |
| **Adapters** | Low | Minimal inline tests; relies on integration tests |
| **Ports (API routes)** | Low | Route handlers not explicitly tested in code |
| **Integration** | Medium | Mock test service available for endpoint testing |

### Areas Lacking Test Coverage
- **API endpoint handlers** — REST/RPC/MCP routes
- **Router logic** — No inline tests found in `ports/api/src/router/`
- **Database operations** — Repositories rely on integration tests
- **Email sending** — Notifier adapter has limited test coverage
- **Middleware** — Authentication middleware not explicitly tested

## Gate Check Commands

### Format Check
```bash
cargo fmt --all -- --check
```
- Validates code follows Rust formatting standards
- Enforced in CI

### Build Check
```bash
cargo build --workspace
```
- Ensures all code compiles
- Checks all workspace members

### Test Check
```bash
cargo test --workspace --all
```
- Runs all unit and integration tests
- Must pass before merge

### Clippy Lint Check
```bash
cargo clippy --workspace --all-targets
```
- Checks for common mistakes and style issues
- Not explicitly mentioned but inferred from CI patterns

### Full CI Check (Local)
Simulate the full CI pipeline:
```bash
# Format
cargo fmt --all -- --check

# Build
cargo build --workspace

# Test
cargo test --workspace --all

# Clippy (if available)
cargo clippy --workspace --all-targets -- -D warnings
```

## Development Workflow

### Adding a Test
1. Identify the function/method to test
2. Add `#[cfg(test)]` module at the end of the file:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[tokio::test]
       async fn test_my_function() -> Result<(), MappedErrors> {
           // test code
           Ok(())
       }
   }
   ```
3. Run locally: `cargo test --package <package_name> test_my_function`
4. Verify CI passes when pushing

### Testing Database Code
For repository tests (diesel adapters):
1. Use in-memory database (mem_db) for unit tests
2. Use integration tests with real PostgreSQL for integration testing
3. Mock `DbPoolProvider` using mockall for isolation

### Testing API Endpoints
1. Use integration test service (`test/downstream_service/`)
2. Start the service: `scripts/run-test-servers.sh`
3. Make HTTP requests to test endpoints
4. Verify response codes and structure

## Known Testing Limitations

1. **No Explicit API Tests** — REST/RPC endpoints lack inline tests
   - Workaround: Use integration test service for manual testing

2. **Limited Repository Tests** — Adapter layer has low test coverage
   - Reason: Requires database setup; integration tests used instead

3. **No E2E Tests** — No full workflow tests (auth → create user → access)
   - Recommendation: Add integration tests for critical paths

4. **Mock Service Only** — Integration tests use simplified mock service
   - Limitation: Doesn't test full gateway routing complexity

## Test Resource Files

### Configuration Test File
- **Location**: `lib/config/tests/config.toml`
- **Used by**: `load_config_from_file()` tests
- **Content**: Sample TOML with variables and SecretResolver fields

### Environment Setup for Tests
- Set environment variables: `std::env::set_var()`
- Create temporary files: Use standard `std::fs` within tests
- Database: Use in-memory adapter or Docker container (CI)

## Future Testing Improvements

### Recommended Additions
1. Add API endpoint tests using `test-actix` or `actix-rt::test`
2. Add repository tests with embedded SQLite for unit testing
3. Add full E2E tests with real PostgreSQL container
4. Increase use case orchestration test coverage
5. Add performance/load tests for gateway routing

### Tools to Consider
- `test-actix` — Actix-web testing utilities
- `sqlx::sqlite` — Embedded test database
- `testcontainers` — Docker container management for tests
- `criterion` — Benchmarking framework

