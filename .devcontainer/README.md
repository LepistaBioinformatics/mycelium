# Dev Container – Mycelium

## Docker Compose profiles

Services are grouped into **profiles**. Only services in the active profile(s) are started.

| Profile         | Services                                                                 | Purpose              |
|-----------------|---------------------------------------------------------------------------|----------------------|
| **default**     | Postgres, Redis, myc-test-service, myc-devcontainer                       | Base development     |
| **observability** | Jaeger, Prometheus, Grafana, OpenTelemetry Collector                    | Metrics and tracing  |
| **security**    | Vault                                                                     | Secrets and security |

### Enabling the default profile (recommended)

By default Compose does **not** activate any profile. For the devcontainer to start with Postgres, Redis, and test-service, you must set `COMPOSE_PROFILES`.

**At the project root**, create or edit the `.env` file and add:

```bash
# Services that start when opening the devcontainer (always include default)
COMPOSE_PROFILES=default
```

The tool that opens the container (VS Code/Cursor) typically runs `docker compose` from the project root; Compose reads the root `.env` and uses this value.

### Enabling other profiles

Use multiple profiles separated by commas in the same `.env`:

```bash
# Base development only
COMPOSE_PROFILES=default

# Base + observability (Jaeger, Prometheus, Grafana, OTEL)
COMPOSE_PROFILES=default,observability

# Base + Vault
COMPOSE_PROFILES=default,security

# All profiles
COMPOSE_PROFILES=default,observability,security
```

Reopen the devcontainer after changing `.env` for the new profiles to take effect.

### Container variables

Variables that are passed **into** the containers (e.g. `MYC_VAULT_TOKEN`, `MYC_REDIS_PASS`) come from the `.env` file **inside `.devcontainer`** (due to `env_file` in `docker-compose.yaml`).

* Copy `.devcontainer/.env.example-with-vaut` to `.devcontainer/.env` and fill in the values.
* `COMPOSE_PROFILES` must be set in the **project root** `.env` (as above), not in `.devcontainer/.env`.

## Summary

1. **Project root** – `.env` with `COMPOSE_PROFILES=default` (and other profiles if needed).
2. **`.devcontainer/.env`** – created from `.env.example-with-vaut` with `MYC_VAULT_TOKEN`, `MYC_REDIS_PASS`, etc.

With this, when you open the devcontainer, the `default` profile (and any others you configured) will be used and the correct services will start.
