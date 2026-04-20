# Running Tests

Tests require PostgreSQL and Redis running. Use Docker Compose to start them:

```bash
docker-compose up -d postgres redis
```

---

## Run all tests

From `modules/mycelium-api-gateway/`:

```bash
cargo test
```

With logs visible:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

---

## Filtering tests

```bash
cargo test auth              # all tests with "auth" in the name
cargo test -p mycelium-base  # specific workspace package
cargo test --workspace       # every package
```

---

## Pre-commit checks

These must all pass before merging:

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all
```

---

## Coverage (optional)

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

---

## Test database setup

```bash
psql postgres://postgres:postgres@localhost:5432/postgres \
  -c "CREATE DATABASE mycelium_test;"
```

Set the env var before running tests that need a separate database:

```bash
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5432/mycelium_test"
```

---

## Troubleshooting

**Tests hang** — run with `--test-threads=1` to serialize them.

**Flaky tests** — run the failing test in isolation: `cargo test <test_name> -- --nocapture`.

**Port conflict** — stop any running `myc-api` process or Docker containers on the same ports.
