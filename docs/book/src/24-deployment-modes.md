# Deployment Modes

Mycelium ships as a single codebase that compiles into one of **three
mutually-exclusive build modes**, selected at **compile time** via Cargo
features. All three expose the identical REST/JSON-RPC API, authentication
flows, webhooks, and MCP integration — they share the same `core` domain logic
and differ only in the adapters wired in `ports/api/src/main.rs`'s
`initialize_modules`. What actually changes between them is **how much external
infrastructure each one requires**.

| | [Full](./25-full-mode.md) | [Postgres-Only](./26-postgres-only-mode.md) | [Standalone](./23-standalone-mode.md) |
|---|---|---|---|
| Cargo feature | `full` *(default)* | `postgres-only` | `standalone` |
| Persistence | PostgreSQL | PostgreSQL | SQLite (embedded, auto-provisioned) |
| Cache / KV | Redis | PostgreSQL (`kv_artifact` table) | in-process ([moka](https://docs.rs/moka)) |
| Email delivery | SMTP | SMTP | stub / file (SMTP opt-in) |
| Secrets | operator-provided / Vault | operator-provided / Vault | auto-generated + persisted |
| **External services** | **PostgreSQL + Redis + SMTP** | **PostgreSQL (+ SMTP)** | **none** |
| Horizontal scaling (multi-pod) | ✅ | ✅ | ❌ single instance |
| Config example | `config.full.example.toml` | `config.postgres-only.example.toml` | `config.standalone.example.toml` |

Each mode's Cargo feature name matches its shipped `config.<mode>.example.toml`.

---

## Choosing a mode

- **[Full](./25-full-mode.md)** — the default build and the published Docker
  image. Use it when you already run Redis, or want Redis-backed caching in a
  multi-pod deployment. This is the historical production topology.
- **[Postgres-Only](./26-postgres-only-mode.md)** — production-grade and
  horizontally scalable like full mode, but with **one less service to operate**:
  no Redis. The KV/artifact cache lives in a PostgreSQL table and the email
  queue is claimed multi-pod-safe on the existing `message_queue` table. Pick it
  when you want a multi-pod deployment backed by PostgreSQL alone.
- **[Standalone](./23-standalone-mode.md)** — **zero external dependencies**
  (embedded SQLite, in-process cache, stub/file email). Single instance only.
  Pick it for local development, evaluation, edge, or air-gapped deployments.

---

## How selection works

Exactly one of the three backend features must be enabled. `full` is the Cargo
**default**, so `cargo build` (no flags) and the published image produce full
mode. The other two are opt-in and require `--no-default-features`, because
Cargo features are additive and the default `full` would otherwise stay on:

```bash
# Full (default)
cargo build --release -p mycelium-api

# Postgres-only
cargo build --release --no-default-features --features postgres-only -p mycelium-api

# Standalone
cargo build --release --no-default-features --features standalone -p mycelium-api
```

Enabling two backend features at once (or none) is a hard `compile_error!` — the
modes select different persistence adapters and Diesel column types that cannot
coexist in one binary. Add `,rhai` to any of the above to also compile the Rhai
scripting support.

---

## A note on schema migrations

`full` and `postgres-only` share the same PostgreSQL schema. The `postgres-only`
mode adds a `kv_artifact` cache table and an index on `message_queue`; those
migrations must be applied to the database (they are harmless in full mode). See
the [Postgres-Only Mode](./26-postgres-only-mode.md#database-migrations) page for
the exact files and commands. Standalone auto-provisions and migrates its SQLite
file on first boot, so it needs no manual step.
