# CLI Reference

The `myc-cli` binary provides commands that must be run directly against the database. These
operations cannot be performed through the HTTP API because they are bootstrapping steps that
execute before any admin account exists.

---

## Installation

`myc-cli` is built alongside the gateway. After building from source:

```bash
cargo build --release
# Binary at: target/release/myc-cli
```

Or install directly:

```bash
cargo install --path ports/cli
```

---

## Database connection

All CLI commands connect directly to PostgreSQL. Provide the connection URL by setting the
`DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgres://user:pass@localhost:5432/mycelium"
```

If `DATABASE_URL` is not set, the CLI prompts you to enter it interactively (input is hidden).

---

## Commands

### `accounts create-seed-account`

Creates the first **Staff** account in a fresh installation. This account is used to log in
and perform all subsequent provisioning (create tenants, invite admins, etc.).

```
myc-cli accounts create-seed-account <email> <account_name> <first_name> <last_name>
```

**Arguments:**

| Argument | Description |
|---|---|
| `email` | Email address for the new account (used to log in) |
| `account_name` | Display name for the account (e.g. the organization name) |
| `first_name` | User's first name |
| `last_name` | User's last name |

**Interactive prompt:** After the positional arguments, the CLI prompts for a password (hidden input).

**Example:**

```bash
myc-cli accounts create-seed-account \
  admin@acme.com \
  "ACME Platform" \
  Alice \
  Smith
# Password: (hidden)
```

**Notes:**
- If a seed staff account already exists, the command exits with an informational message and
  does not create a duplicate.
- The created account has the `Staff` type (platform-wide administrative privileges).
- After creation, use the magic-link or password login flow to authenticate and start
  provisioning tenants.

---

### `native-errors init`

Seeds the database with all native Mycelium error codes. These are the error codes that the
core domain layer emits internally (prefixed `MYC`). Without this step, error responses that
carry domain codes will have no human-readable message.

```
myc-cli native-errors init
```

**No arguments.** The command reads `DATABASE_URL` (or prompts interactively) and inserts all
built-in error codes. Codes that already exist are skipped; only new codes are inserted.

**When to run:** Once, during initial installation, and again after upgrading to a new version
of Mycelium that introduces new error codes.

**Example:**

```bash
DATABASE_URL="postgres://user:pass@localhost:5432/mycelium" myc-cli native-errors init
# INFO: 42 native error codes registered
```

---

## Typical installation order

```bash
# 1. Apply the database schema
psql "$DATABASE_URL" -f postgres/sql/up.sql

# 2. Seed native error codes
myc-cli native-errors init

# 3. Create the first admin account
myc-cli accounts create-seed-account admin@example.com "My Platform" Admin User

# 4. Start the API server
SETTINGS_PATH=settings/config.toml myc-api
```

After step 4, log in with `admin@example.com` and the password you set in step 3.
