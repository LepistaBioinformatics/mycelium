# Full Mode

Full mode is the **default build** and the published Docker image. It runs the
complete external stack — **PostgreSQL + Redis + SMTP** (plus optional
HashiCorp Vault) — and is the historical production topology. See
[Deployment Modes](./24-deployment-modes.md) for how it compares to the other
two modes.

---

## What it uses

| Concern | Backing service | Adapter |
|---|---|---|
| Persistence | PostgreSQL | `adapters/diesel_postgres` |
| Cache / KV | Redis (`SETEX`, TTL'd) | `adapters/kv_db` |
| Email delivery | SMTP (`lettre`) | `adapters/notifier` |
| Email queue | PostgreSQL `message_queue` table, polled by the in-process dispatcher | `adapters/diesel_postgres` |
| Secrets | operator-provided (`{ env = ... }`) or HashiCorp Vault | `lib/config` |

Redis is used as the TTL'd artifact cache (JWKS, profile, and email caches). It
is a **required** service — the gateway will not boot without a reachable Redis
and a `[redis]` config section.

---

## Building and running

```bash
# Default build — `full` is the default Cargo feature.
cargo build --release -p mycelium-api

SETTINGS_PATH=./config.toml myc-api
```

Start from `settings/config.full.example.toml`. A full-mode config requires
`[core.accountLifeCycle]`, `[diesel]`, `[redis]`, `[smtp]`, `[queue]`, `[auth]`,
and `[api]`; `[vault]` is optional.

```toml
[diesel]
databaseUrl = { env = "DATABASE_URL" }

[redis]
protocol = "redis"        # rediss for TLS
hostname = "my-redis"
password = { env = "REDIS_PASSWORD" }

[smtp]
host = "smtp.example.com"
username = { env = "SMTP_USERNAME" }
password = { env = "SMTP_PASSWORD" }
port = 465
```

### Docker

The published image is full mode with no extra flags:

```bash
docker build -t mycelium-api .
docker run -p 8080:8080 -e SETTINGS_PATH=/config.toml -v ./config.toml:/config.toml mycelium-api
```

The build features are parameterized via the `CARGO_FEATURES` build-arg, which
defaults to `full,rhai` — so the default image is unchanged. (Override it only
to build a different mode; see [Postgres-Only Mode](./26-postgres-only-mode.md).)

---

## When to use it

- You already operate Redis and want its cache in a multi-pod deployment.
- You want the exact topology the published image ships.

If you run multiple replicas but would rather **not** operate Redis, use
[Postgres-Only Mode](./26-postgres-only-mode.md), which keeps PostgreSQL and
multi-pod scaling but drops Redis. For a single-instance, zero-dependency build,
use [Standalone Mode](./23-standalone-mode.md).
