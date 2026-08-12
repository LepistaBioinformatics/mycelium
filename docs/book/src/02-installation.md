# Installation

Mycelium API Gateway is distributed as a single binary (`myc-api`). Pick the installation method
that fits your workflow.

---

## Prerequisites

You need three services running before Mycelium can start:

| Service | Minimum version | Purpose |
|---|---|---|
| PostgreSQL | 14 | Stores users, tenants, roles |
| Redis | 6 | Caching layer |
| Rust toolchain | 1.70 (build from source only) | Compiles the binary |

Install Rust via [rustup](https://rustup.rs/) if you plan to build from source.

**Linux system dependencies** (Ubuntu/Debian):
```bash
sudo apt-get install -y build-essential pkg-config libssl-dev postgresql-client
```

**macOS:**
```bash
brew install openssl pkg-config postgresql
```

---

## Option A — Docker (fastest)

```bash
docker pull ghcr.io/LepistaBioinformatics/mycelium:latest
```

For a full local environment with PostgreSQL and Redis already wired up, see
[Deploy Locally](./05-deploy-locally.md).

---

## Option B — Install via Cargo

```bash
cargo install mycelium-api
```

This installs the `myc-api` binary globally. Verify it:

```bash
myc-api --version
```

---

## Option C — Build from source

```bash
git clone https://github.com/LepistaBioinformatics/mycelium.git
cd mycelium
cargo build --release
./target/release/myc-api --version
```

---

## Database setup

Mycelium ships with a single SQL script that creates the database, user, and the complete
schema in one invocation:

```bash
psql postgres://postgres:postgres@localhost:5432/postgres \
  -f adapters/diesel_postgres/sql/up.sql \
  -v db_password='REPLACE_WITH_STRONG_PASSWORD'
```

This creates a database named `mycelium-dev` and a user named `mycelium-user`. To use a
different database name, add `-v db_name='my-database'`.

There is nothing else to apply — every migration under
`adapters/diesel_postgres/sql/migrations/` is already folded into `up.sql`. The DDL runs in
a single transaction, so a failure leaves no partial schema behind.

`up.sql` performs **fresh installs only**. Run against a database that already has the
schema, it prints a notice and exits without changing anything. To upgrade an existing
`9.0.0-rc.x` database, apply the files in
`adapters/diesel_postgres/sql/migrations/` in chronological order instead — **as the same
superuser that created the schema**, not as the application's own user. Some of those files
issue `GRANT`s, which require table ownership.

> Upgrading from `9.0.0-rc.x`? Apply `20260812_01_audit_tables_grants.sql` even if you think
> you are current. Until it lands, `instance_settings` and `resource_audit_log` carry no
> grants for the application role, and the staff-bootstrap claim and audit-log writes fail
> for any deployment whose app does not connect as a superuser.

> **SQLite / standalone mode needs none of this.** That backend compiles its migrations
> into the binary and applies them on every boot, so the schema appears when you start
> `myc-api`.

---

## Next steps

- [Quick Start](./03-quick-start.md) — Start the gateway with a minimal config
- [Deploy Locally](./05-deploy-locally.md) — Full Docker Compose setup with all dependencies

---

## Troubleshooting

**`cargo install` fails with SSL errors** — Install OpenSSL dev libraries:
`sudo apt-get install libssl-dev` (Ubuntu) or `brew install openssl` (macOS).

**Database connection fails** — Verify PostgreSQL is running: `psql --version` and
`psql postgres://postgres:postgres@localhost:5432/postgres`.

**Redis not responding** — Run `redis-cli ping`. Expect `PONG`.
