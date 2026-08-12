# Postgres-Only Mode

Postgres-only mode runs the full production feature set — PostgreSQL
persistence, real SMTP email, and **multi-pod horizontal scaling** — but with
**no Redis**. Both jobs Redis does in [full mode](./25-full-mode.md) move to
PostgreSQL: the KV/artifact cache becomes a table, and the email queue is
claimed multi-pod-safe on the existing `message_queue` table. Pick it when you
want a horizontally-scalable deployment backed by PostgreSQL alone, without
operating Redis. See [Deployment Modes](./24-deployment-modes.md) for the full
comparison.

---

## What changes

| Full mode | Postgres-only mode |
|---|---|
| Redis-backed cache (`adapters/kv_db`, `SETEX`) | PostgreSQL `kv_artifact` table (`adapters/postgres_kv`) |
| Redis client + `[redis]` config required | No Redis; `[redis]` is not read (a present section is ignored) |
| Email queue coordinated across pods via Redis | Email queue claimed on `message_queue` via `SELECT … FOR UPDATE SKIP LOCKED` |
| SMTP delivery | SMTP delivery (unchanged) |

Everything else — the REST/JSON-RPC API, auth flows, webhooks, MCP,
PostgreSQL persistence, and Vault/secret resolution — is identical to full
mode. Only the cache adapter and the email-claim query differ.

The Postgres KV cache stores the same TTL'd artifacts Redis did (JWKS, profile,
email caches): `set_encoded_artifact` upserts `(key, value, expires_at = now()
+ ttl)`, reads filter on `expires_at > now()` (lazy expiry), and a background
sweeper reclaims expired rows. The cache is shared across pods (one table),
matching the cross-instance coherence Redis provided.

---

## Building and running

```bash
cargo build --release --no-default-features --features postgres-only -p mycelium-api

SETTINGS_PATH=./config.toml myc-api
```

`--no-default-features` is required — Cargo features are additive, and the
default `full` feature would otherwise stay enabled and trip the
mutual-exclusion `compile_error!` guard. Add `,rhai` for Rhai scripting.

Start from `settings/config.postgres-only.example.toml`. The config needs
`[core.accountLifeCycle]`, `[diesel]`, `[smtp]`, `[queue]`, `[auth]`, and
`[api]` — **no `[redis]`** (this build never reads it). `[vault]` is optional.

```toml
[diesel]
databaseUrl = { env = "DATABASE_URL" }

[smtp]
host = "smtp.example.com"
username = { env = "SMTP_USERNAME" }
password = { env = "SMTP_PASSWORD" }
port = 465
# No [redis] section.
```

### Docker

Reuse the main `Dockerfile` with the `CARGO_FEATURES` build-arg (the default
image stays full mode; overriding the arg builds postgres-only):

```bash
docker build --build-arg CARGO_FEATURES=postgres-only,rhai -t mycelium-api-postgres-only .
```

> `cargo install` from crates.io only works once `mycelium-postgres-kv` and the
> `postgres-only` feature are published. Until then, build the image from source
> (`cargo build --no-default-features --features postgres-only,rhai`).

---

## Database migrations

`full` and `postgres-only` share the same PostgreSQL schema. Postgres-only adds
two objects; **apply them before running this build** or boot/queries fail on
the missing objects. Fresh installs get them from `sql/up.sql`; existing
databases apply the incremental files (operator-applied via `psql`, as with all
PostgreSQL migrations here):

```bash
psql "$DATABASE_URL" -f adapters/diesel_postgres/sql/migrations/20260722_01_kv_artifact_cache.sql
psql "$DATABASE_URL" -f adapters/diesel_postgres/sql/migrations/20260722_02_message_queue_claim_index.sql
```

- **`20260722_01`** — the `kv_artifact` cache table + `idx_kv_artifact_expires_at`. Used only by postgres-only; harmless and empty in full mode.
- **`20260722_02`** — `idx_message_queue_claim` on `message_queue (status, created)`, backing the claim query below. **Applies to full mode too**, since the claim change is shared.

---

## Email queue and multi-pod safety

Both full and postgres-only deliver email by polling the PostgreSQL
`message_queue` table with an in-process dispatcher. Postgres-only must run
multiple pods, so the dispatcher claims pending messages atomically:

```sql
UPDATE message_queue SET attempted = now()
WHERE id IN (
  SELECT id FROM message_queue
  WHERE status = 'Queued'
    AND (attempted IS NULL OR attempted < now() - <visibility>)
  ORDER BY created DESC
  LIMIT <batch> FOR UPDATE SKIP LOCKED
) RETURNING …;
```

`SKIP LOCKED` guarantees each message is claimed by at most one pod (no
double-send); a claimed message becomes re-claimable after a visibility timeout,
so a crashed pod's message is not lost. This claim also fixes a pre-existing
full-mode double-send, so the change applies to full mode as well.

Two `[queue]` fields tune it (both honored in full mode):

```toml
[queue]
claimBatchSize = 3          # messages claimed per dispatcher tick
visibilityTimeoutSecs = 240 # how long a claimed batch stays invisible to other pods
```

> **Safety invariant:** `visibilityTimeoutSecs > claimBatchSize × worst-case
> single SMTP send`. The defaults (batch 3, window 240s) are safe for lettre's
> ~60s default send timeout. The window also doubles as the retry back-off for a
> transiently-failed send — lower it for faster retries (e.g. time-sensitive OTP
> email) **only after** lowering the batch so the invariant still holds.

---

## Known limitations / trade-offs

- **Cache reads/writes hit PostgreSQL.** Redis exists precisely to keep these
  off the hot database path; in postgres-only they add load to your primary
  store. Acceptable for most workloads, but size the database accordingly.
- **Failed-email retry latency equals `visibilityTimeoutSecs`.** Claim-safety
  and retry back-off are the same knob (both key off the `attempted` column), so
  they cannot be independently short. Tune via `[queue]`.
- **No Vault requirement change.** Secrets resolve exactly as in full mode
  (operator-provided or Vault); postgres-only does **not** auto-generate secrets
  the way [standalone](./23-standalone-mode.md) does.
