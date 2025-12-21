# Running Tests

[🏠 Home](/README.md)

[📋 Summary](/docs/book/src/SUMMARY.md)

This guide explains how to run the test suite for Mycelium API Gateway. Mycelium includes comprehensive unit tests, integration tests, and end-to-end tests to ensure code quality and reliability.

## Prerequisites

Before running tests, ensure you have:

- **Rust toolchain** (version 1.70 or higher)
- **Development dependencies** installed
- **Test environment** configured (Postgres, Redis running)

## Quick Start

Run all tests with a single command:

```bash
cargo test
```

This will compile and run all unit tests, integration tests, and documentation tests in the project.

## Test Categories

Mycelium's test suite is organized into several categories:

### Unit Tests

Unit tests verify individual functions and modules in isolation.

**Run all unit tests:**
```bash
cargo test --lib
```

**Run tests for a specific module:**
```bash
cargo test --lib users
cargo test --lib auth
cargo test --lib routing
```

**Run a specific test:**
```bash
cargo test --lib test_user_creation
```

### Integration Tests

Integration tests verify interactions between multiple components.

**Run all integration tests:**
```bash
cargo test --test '*'
```

**Run a specific integration test:**
```bash
cargo test --test api_integration
```

### Documentation Tests

Documentation tests ensure code examples in documentation are correct.

**Run documentation tests:**
```bash
cargo test --doc
```

## Test Environment Setup

Some tests require external services (Postgres, Redis). You can set these up in multiple ways:

### Option 1: Using Docker Compose (Recommended)

Start test dependencies using Docker Compose:

```bash
# Start test services
docker-compose -f docker-compose.test.yaml up -d

# Run tests
cargo test

# Stop test services
docker-compose -f docker-compose.test.yaml down
```

### Option 2: Local Services

If you have Postgres and Redis running locally:

```bash
# Set test database URL
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5432/mycelium_test"

# Set test Redis URL
export TEST_REDIS_URL="redis://localhost:6379"

# Run tests
cargo test
```

### Option 3: Using Test Containers

Mycelium supports testcontainers for automatic service provisioning:

```bash
# Tests will automatically start and stop required containers
cargo test --features testcontainers
```

## Running Specific Tests

### By Test Name

Run tests matching a pattern:

```bash
# Run all tests with "auth" in the name
cargo test auth

# Run all tests with "user_service" in the name
cargo test user_service
```

### By Package

Run tests for a specific workspace package:

```bash
# Test the API package
cargo test -p mycelium-api

# Test the base package
cargo test -p mycelium-base

# Test all packages
cargo test --workspace
```

### By Feature

Run tests for specific features:

```bash
# Test with all features enabled
cargo test --all-features

# Test without default features
cargo test --no-default-features

# Test with specific features
cargo test --features "vault,oauth2"
```

## Test Output and Debugging

### Verbose Output

See detailed test output:

```bash
cargo test -- --nocapture
```

### Show Test Output (Even for Passing Tests)

```bash
cargo test -- --show-output
```

### Run Tests in Single Thread (Useful for Debugging)

```bash
cargo test -- --test-threads=1
```

### Enable Logging During Tests

```bash
RUST_LOG=debug cargo test -- --nocapture
```

## Code Coverage

Generate code coverage reports using `tarpaulin`:

### Install tarpaulin

```bash
cargo install cargo-tarpaulin
```

### Generate Coverage Report

```bash
# Generate coverage and display in terminal
cargo tarpaulin --out Stdout

# Generate HTML coverage report
cargo tarpaulin --out Html

# Generate coverage for CI (Codecov, Coveralls)
cargo tarpaulin --out Xml
```

### View Coverage Report

```bash
# Open HTML report
open tarpaulin-report.html  # macOS
xdg-open tarpaulin-report.html  # Linux
```

## Benchmarking

Run performance benchmarks:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench routing_benchmark
```

## Continuous Integration

Mycelium includes CI configurations for automated testing:

### GitHub Actions

The repository includes GitHub Actions workflows that run on every push and pull request:

- Unit tests
- Integration tests
- Code coverage
- Linting (clippy)
- Formatting check

View workflow status:
```bash
# Check recent workflow runs
gh run list

# View specific run
gh run view <run-id>
```

### Running CI Tests Locally

Simulate CI environment locally:

```bash
# Run all checks that CI runs
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all-features
```

## Test Database Management

### Initialize Test Database

```bash
# Create test database
psql postgres://postgres:postgres@localhost:5432/postgres \
  -c "CREATE DATABASE mycelium_test;"

# Run migrations
diesel migration run --database-url postgres://postgres:postgres@localhost:5432/mycelium_test
```

### Reset Test Database

```bash
# Drop and recreate test database
psql postgres://postgres:postgres@localhost:5432/postgres \
  -c "DROP DATABASE IF EXISTS mycelium_test;"
psql postgres://postgres:postgres@localhost:5432/postgres \
  -c "CREATE DATABASE mycelium_test;"

# Run migrations again
diesel migration run --database-url postgres://postgres:postgres@localhost:5432/mycelium_test
```

## Common Test Commands

### Fast Test Cycle (Development)

```bash
# Run tests in watch mode (requires cargo-watch)
cargo install cargo-watch
cargo watch -x test
```

### Pre-Commit Checks

```bash
# Run all checks before committing
cargo fmt --check && \
cargo clippy -- -D warnings && \
cargo test --all-features
```

### Release Build Tests

```bash
# Test with release optimizations
cargo test --release
```

## Test Organization

Mycelium tests are organized following Rust conventions:

```
mycelium/
├── src/
│   └── lib.rs              # Unit tests in modules
├── tests/
│   ├── api_tests.rs        # Integration tests
│   ├── auth_tests.rs
│   └── routing_tests.rs
└── benches/
    └── routing_bench.rs    # Benchmarks
```

## Writing New Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("test@example.com");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Test Example

```rust
// tests/api_tests.rs
use mycelium_api::*;

#[tokio::test]
async fn test_health_endpoint() {
    let app = setup_test_app().await;
    let response = app.get("/health").await;
    assert_eq!(response.status(), 200);
}
```

## Troubleshooting

### Tests Fail Due to Database Connection

**Issue:** Cannot connect to test database

**Solution:**
```bash
# Verify Postgres is running
pg_isready

# Check connection string
echo $TEST_DATABASE_URL
```

### Tests Hang or Timeout

**Issue:** Tests hang indefinitely

**Solution:**
```bash
# Run with timeout
cargo test -- --test-threads=1 --timeout 30
```

### Flaky Tests

**Issue:** Tests fail intermittently

**Solution:**
```bash
# Run tests multiple times to reproduce
for i in {1..10}; do cargo test test_name || break; done
```

### Port Conflicts

**Issue:** Test services can't bind to ports

**Solution:**
```bash
# Stop conflicting services
docker-compose down
killall myc-api
```

## Test Best Practices

1. **Isolate tests**: Each test should be independent
2. **Use test fixtures**: Create reusable test data
3. **Clean up**: Tests should clean up after themselves
4. **Mock external services**: Use mocks for external dependencies
5. **Test edge cases**: Don't just test the happy path
6. **Keep tests fast**: Optimize slow tests
7. **Use descriptive names**: Test names should describe what they test

## Next Steps

- [Contributing Guidelines](https://github.com/LepistaBioinformatics/mycelium/blob/main/CONTRIBUTING.md) - Learn how to contribute
- [Configuration Guide](./04-configuration.md) - Configure your development environment
- [Authorization Model](./01-authorization.md) - Understand the security model

## Additional Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [GitHub Issues](https://github.com/LepistaBioinformatics/mycelium/issues) - Report test failures

---

<div style="display: flex; justify-content: space-between; align-items: center; margin-top: 2rem; padding-top: 1rem; border-top: 1px solid #e0e0e0;">
  <a href="./06-downstream-apis.md" style="text-decoration: none; color: #0066cc;">◀️ Previous: Downstream APIs</a>
  <span></span>
</div>
