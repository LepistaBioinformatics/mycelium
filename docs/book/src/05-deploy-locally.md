# Deploy Locally with Docker Compose

The fastest way to get a complete development environment is Docker Compose — it starts
PostgreSQL, Redis, Vault, and the gateway together with one command.

**Prerequisites:** Docker 20.10+ and Docker Compose 2.0+.
([Docker Desktop](https://docs.docker.com/desktop/) includes both.)

---

## Step 1 — Clone and configure

```bash
git clone https://github.com/LepistaBioinformatics/mycelium.git
cd mycelium
cp settings/config.example.toml settings/config.toml
```

Open `settings/config.toml` and update at minimum:
- Database credentials under `[diesel]`
- SMTP settings under `[smtp]` (if you need email)
- Secrets under `[core.accountLifeCycle]` and `[auth]`

---

## Step 2 — Start everything

```bash
docker-compose up -d
```

This starts:
- **postgres** — database on port 5432
- **redis** — cache on port 6379
- **vault** — secret management on port 8200 (optional)
- **mycelium-api** — gateway on port 8080

---

## Step 3 — Verify

```bash
docker-compose ps        # all services should be "Up"
curl http://localhost:8080/health
```

---

## Common operations

**View logs:**
```bash
docker-compose logs -f mycelium-api
```

**Stop everything:**
```bash
docker-compose down
```

**Full reset** (deletes all data):
```bash
docker-compose down -v
```

**Access the database directly:**
```bash
docker-compose exec postgres psql -U mycelium-user -d mycelium-dev
```

---

## Using Vault for secrets (optional)

If you're using Vault, initialize it after starting:

```bash
# Initialize and get unseal keys + root token (save these securely)
docker-compose exec vault vault operator init

# Unseal with 3 of the 5 keys
docker-compose exec vault vault operator unseal <KEY1>
docker-compose exec vault vault operator unseal <KEY2>
docker-compose exec vault vault operator unseal <KEY3>

# Store a secret
docker-compose exec vault vault login <ROOT_TOKEN>
docker-compose exec vault vault kv put secret/mycelium/database \
  url="postgres://mycelium-user:password@postgres:5432/mycelium"
```

Then reference it in `config.toml`:
```toml
[vault.define]
url = "http://vault:8200"
versionWithNamespace = "v1/secret"
token = { env = "VAULT_TOKEN" }

[diesel]
databaseUrl = { vault = { path = "mycelium/database", key = "url" } }
```

---

## Troubleshooting

**Gateway can't connect to Postgres:**
```bash
docker-compose exec postgres pg_isready
docker-compose exec postgres psql -U postgres -l
```

**Port conflict** — change the external port in `docker-compose.yaml`:
```yaml
services:
  mycelium-api:
    ports:
      - "8081:8080"
```

---

## Next steps

- [Configuration](./04-configuration.md) — Full config reference
- [Downstream APIs](./06-downstream-apis.md) — Register your backend services
