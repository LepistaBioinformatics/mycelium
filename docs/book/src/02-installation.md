# Installation Guide

[🏠 Home](/README.md)

[📋 Summary](/docs/book/src/SUMMARY.md)

This guide will walk you through the steps to install Mycelium API Gateway on your local machine. Mycelium API Gateway package includes twelve libs and services, available in Crates.io. It should be installed using the cargo package manager.

## Prerequisites

Before you start, make sure you have the following installed on your machine:

### Required

- **Rust** (version 1.70 or higher)
  - Install via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Postgres** (version 14 or higher)
  - Database for tenant and user management
  - Installation: [PostgreSQL Downloads](https://www.postgresql.org/download/)
- **Redis** (version 6 or higher)
  - Caching for performance
  - Installation: [Redis Downloads](https://redis.io/download)

### Optional but Recommended

- **HashiCorp Vault**
  - Recommended for secret management in production environments
  - Installation: [Vault Installation](https://developer.hashicorp.com/vault/tutorials/getting-started/getting-started-install)
- **Docker & Docker Compose**
  - Optional for quick deployment and development
  - Installation: [Docker Desktop](https://www.docker.com/products/docker-desktop)

### System Dependencies

Depending on your operating system, you may need additional system dependencies:

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev postgresql-client
```

**macOS:**
```bash
brew install openssl pkg-config postgresql
```

**Fedora/RHEL:**
```bash
sudo dnf install -y gcc openssl-devel postgresql
```

## Installation Methods

### Method 1: Install using Cargo (Recommended for Production)

The simplest way to install Mycelium API Gateway is to use the `cargo` package manager:

```bash
cargo install mycelium-api
```

This will install the `myc-api` binary globally on your system.

### Method 2: Build from Source

If you want to build from source or contribute to the project:

1. Clone the repository:
```bash
git clone https://github.com/LepistaBioinformatics/mycelium.git
cd mycelium
```

2. Build the project:
```bash
cargo build --release
```

3. The binary will be available at `target/release/myc-api`

### Method 3: Using Docker

The easiest way to get started quickly is to use the Docker image:

```bash
docker pull sgelias/mycelium-api:latest
```

For development with docker-compose, see the [Deploy Locally](./05-deploy-locally.md) guide.

## Database Setup

Mycelium API Gateway uses Postgres as the main datastore. Follow these steps to initialize the database:

### Step 1: Ensure Postgres is Running

Verify that your Postgres instance is running:

```bash
psql --version
```

### Step 2: Initialize the Database

Use the provided SQL script to initialize the database. The script creates the necessary database, user, and schema:

```bash
psql postgres://postgres:postgres@localhost:5432/postgres \
  -f postgres/sql/up.sql \
  -v db_password='REPLACE_ME'
```

**Important Notes:**
- Replace `postgres://postgres:postgres@localhost:5432/postgres` with your actual Postgres connection string
- Replace `REPLACE_ME` with a strong password for the database user
- The script creates a database named `mycelium-dev` by default

### Step 3: Customize Database Name (Optional)

You can customize the database name using the `db_name` variable:

```bash
psql postgres://postgres:postgres@localhost:5432/postgres \
  -f postgres/sql/up.sql \
  -v db_name='my-mycelium-database' \
  -v db_password='REPLACE_ME'
```

Additional customizations for `db_user` and `db_role` should be done in the `up.sql` script. Default values are:
- **db_user**: `mycelium-user`
- **db_role**: `service-role-mycelium`

## Vault Setup (Optional)

Vault is optional but highly recommended for secret management in production environments.

### Step 1: Initialize Vault

Use the standard Vault CLI commands to initialize and unseal Vault:

```bash
vault operator init
```

You will receive output similar to:

```
Unseal Key 1: REPLACE_ME
Unseal Key 2: REPLACE_ME
Unseal Key 3: REPLACE_ME
Unseal Key 4: REPLACE_ME
Unseal Key 5: REPLACE_ME

Initial Root Token: REPLACE_ME
```

**Important:** Store these keys securely. You will need them to unseal Vault after each restart.

### Step 2: Unseal Vault

Unseal Vault using at least 3 of the unseal keys:

```bash
vault operator unseal
# Enter unseal key 1
vault operator unseal
# Enter unseal key 2
vault operator unseal
# Enter unseal key 3
```

### Step 3: Configure Vault for Mycelium

For detailed information on configuring Vault for use with Mycelium, refer to the [Configuration Guide](./04-configuration.md#vault-configurations).

For more information on Vault, see the official [Vault documentation](https://www.vaultproject.io/docs).

## Verify Installation

To verify that Mycelium is installed correctly:

### If installed via Cargo:
```bash
myc-api --version
```

### If built from source:
```bash
./target/release/myc-api --version
```

### If using Docker:
```bash
docker run --rm sgelias/mycelium-api:latest --version
```

## Next Steps

Now that you have Mycelium installed, proceed to:
- [Quick Start Guide](./03-quick-start.md) - Get your first instance running
- [Configuration Guide](./04-configuration.md) - Learn about configuration options
- [Deploy Locally](./05-deploy-locally.md) - Set up a complete development environment

## Troubleshooting

### Common Issues

**Issue: `cargo install` fails with SSL errors**
- Solution: Ensure OpenSSL development libraries are installed
- Ubuntu/Debian: `sudo apt-get install libssl-dev`
- macOS: `brew install openssl`

**Issue: Database connection fails**
- Solution: Verify Postgres is running and connection string is correct
- Check: `psql postgres://postgres:postgres@localhost:5432/postgres`

**Issue: Redis connection fails**
- Solution: Verify Redis is running
- Check: `redis-cli ping` (should return `PONG`)

**Issue: Permission denied when running `up.sql`**
- Solution: Ensure you have superuser privileges or use a user with sufficient permissions

For more help, visit the [GitHub Issues](https://github.com/LepistaBioinformatics/mycelium/issues) page.

---

<div style="display: flex; justify-content: space-between; align-items: center; margin-top: 2rem; padding-top: 1rem; border-top: 1px solid #e0e0e0;">
  <a href="./00-introduction.md" style="text-decoration: none; color: #0066cc;">◀️ Previous: Introduction</a>
  <a href="./03-quick-start.md" style="text-decoration: none; color: #0066cc;">Next: Quick Start ▶️</a>
</div>
