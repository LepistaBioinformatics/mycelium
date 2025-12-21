# Deploy Locally with Docker Compose

This guide explains how to deploy Mycelium API Gateway locally using Docker Compose. This is the recommended approach for development and testing, as it provides a complete environment with all dependencies.

## Prerequisites

Before starting, ensure you have:

- **Docker** (version 20.10 or higher)
- **Docker Compose** (version 2.0 or higher)

Install Docker Desktop for your platform:
- [Docker Desktop for Mac](https://docs.docker.com/desktop/mac/install/)
- [Docker Desktop for Windows](https://docs.docker.com/desktop/windows/install/)
- [Docker Engine for Linux](https://docs.docker.com/engine/install/)

## Quick Start with Docker Compose

### Step 1: Clone the Repository

```bash
git clone https://github.com/LepistaBioinformatics/mycelium.git
cd mycelium
```

### Step 2: Review and Customize Configuration

The repository includes example configuration files. Copy and customize them:

```bash
# Copy example configuration
cp settings/config.example.toml settings/config.toml

# Review and update the configuration
nano settings/config.toml
```

Important settings to review:
- Database credentials
- Redis configuration
- SMTP settings (if using email notifications)
- Vault configuration (if using Vault)

### Step 3: Start the Services

Start all services using Docker Compose:

```bash
docker-compose up -d
```

This will start:
- **Postgres** - Database for tenant and user management
- **Redis** - Caching layer
- **Vault** - Secret management (optional)
- **Mycelium API Gateway** - The main gateway service

### Step 4: Initialize the Database

Wait for Postgres to be ready, then initialize the database:

```bash
# Wait for Postgres to be ready
docker-compose exec postgres pg_isready

# Initialize the database
docker-compose exec postgres psql -U postgres -f /docker-entrypoint-initdb.d/up.sql \
  -v db_password='mycelium-password'
```

Alternatively, if the SQL script is mounted:
```bash
docker-compose exec -T postgres psql -U postgres < postgres/sql/up.sql
```

### Step 5: Verify Services

Check that all services are running:

```bash
docker-compose ps
```

You should see all services in the "Up" state.

Test the API Gateway health endpoint:

```bash
curl http://localhost:8080/health
```

## Docker Compose Configuration

### Understanding the docker-compose.yaml

The `docker-compose.yaml` file (docker-compose files use YAML format, but Mycelium configuration uses TOML) defines the services required for Mycelium:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:14
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: postgres
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./postgres/sql:/docker-entrypoint-initdb.d

  redis:
    image: redis:7
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data

  vault:
    image: vault:1.13
    ports:
      - "8200:8200"
    environment:
      VAULT_DEV_ROOT_TOKEN_ID: mycelium-dev-token
      VAULT_DEV_LISTEN_ADDRESS: 0.0.0.0:8200
    cap_add:
      - IPC_LOCK

  mycelium-api:
    image: sgelias/mycelium-api:latest
    ports:
      - "8080:8080"
    environment:
      SETTINGS_PATH: /app/settings/config.toml
    volumes:
      - ./settings:/app/settings
    depends_on:
      - postgres
      - redis

volumes:
  postgres-data:
  redis-data:
```

### Customizing Services

You can customize service configurations by modifying `docker-compose.yaml` or creating a `docker-compose.override.yaml` file:

```yaml
# docker-compose.override.yaml
version: '3.8'

services:
  mycelium-api:
    environment:
      LOG_LEVEL: debug
    ports:
      - "8081:8080"  # Change external port
```

## Development Workflow

### Building from Source

If you want to build Mycelium from source instead of using the pre-built image:

1. Update `docker-compose.yaml`:
```yaml
services:
  mycelium-api:
    build:
      context: .
      dockerfile: Dockerfile
    # Remove the 'image' line
```

2. Build and start:
```bash
docker-compose up -d --build
```

### Hot Reload for Development

For development with hot reload, you can mount your source code:

```yaml
services:
  mycelium-api:
    volumes:
      - ./settings:/app/settings
      - ./src:/app/src  # Mount source code
      - ./Cargo.toml:/app/Cargo.toml
    command: cargo watch -x run
```

Then rebuild:
```bash
docker-compose up -d --build
```

### Viewing Logs

View logs for all services:
```bash
docker-compose logs -f
```

View logs for a specific service:
```bash
docker-compose logs -f mycelium-api
docker-compose logs -f postgres
docker-compose logs -f redis
```

### Stopping Services

Stop all services:
```bash
docker-compose down
```

Stop and remove volumes (WARNING: This deletes all data):
```bash
docker-compose down -v
```

## Working with Vault

If using Vault for secret management, initialize it after starting:

### Step 1: Initialize Vault

```bash
docker-compose exec vault vault operator init
```

Save the unseal keys and root token securely.

### Step 2: Unseal Vault

```bash
# Unseal with 3 keys
docker-compose exec vault vault operator unseal <KEY1>
docker-compose exec vault vault operator unseal <KEY2>
docker-compose exec vault vault operator unseal <KEY3>
```

### Step 3: Store Secrets

```bash
# Login with root token
docker-compose exec vault vault login <ROOT_TOKEN>

# Store a secret
docker-compose exec vault vault kv put secret/mycelium/database url="postgres://user:pass@postgres:5432/mycelium"
```

### Step 4: Update Configuration

Update `settings/config.toml` to use Vault:

```toml
[vault.define]
url = "http://vault:8200"
versionWithNamespace = "v1/secret"
token = { env = "VAULT_TOKEN" }

[diesel]
databaseUrl = { vault = { path = "mycelium/database", key = "url" } }
```

## Database Management

### Accessing the Database

Connect to the Postgres database:

```bash
docker-compose exec postgres psql -U mycelium-user -d mycelium-dev
```

### Running Migrations

If you have database migrations:

```bash
docker-compose exec mycelium-api diesel migration run
```

### Backing Up the Database

Create a database backup:

```bash
docker-compose exec -T postgres pg_dump -U mycelium-user mycelium-dev > backup.sql
```

Restore from backup:

```bash
docker-compose exec -T postgres psql -U mycelium-user mycelium-dev < backup.sql
```

## Environment-Specific Configurations

### Development Environment

Create `docker-compose.dev.yaml` (note: docker-compose files remain in YAML format):

```yaml
version: '3.8'

services:
  mycelium-api:
    environment:
      RUST_LOG: debug
      RUST_BACKTRACE: 1
    volumes:
      - ./settings:/app/settings
```

Use it with:
```bash
docker-compose -f docker-compose.yaml -f docker-compose.dev.yaml up -d
```

### Production Environment

For production, use separate configuration:

```bash
docker-compose -f docker-compose.yaml -f docker-compose.prod.yaml up -d
```

## Troubleshooting

### Service won't start

Check the logs:
```bash
docker-compose logs mycelium-api
```

### Database connection issues

Ensure Postgres is ready:
```bash
docker-compose exec postgres pg_isready
```

Check database exists:
```bash
docker-compose exec postgres psql -U postgres -l
```

### Port conflicts

If ports are already in use, modify `docker-compose.yaml`:
```yaml
services:
  mycelium-api:
    ports:
      - "8081:8080"  # Use different external port
```

### Reset Everything

To completely reset your environment:
```bash
docker-compose down -v
docker-compose up -d
# Re-initialize database
```

## Next Steps

- [Configuration Guide](./04-configuration.md) - Detailed configuration options
- [Downstream APIs](./06-downstream-apis.md) - Configure routing to backend services
- [Running Tests](./07-running-tests.md) - Run the test suite

## Additional Resources

- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Mycelium GitHub Repository](https://github.com/LepistaBioinformatics/mycelium)
- [Report Issues](https://github.com/LepistaBioinformatics/mycelium/issues)
