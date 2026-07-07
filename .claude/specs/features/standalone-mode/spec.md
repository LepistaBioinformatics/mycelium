# Feature Spec: Standalone Mode

**Feature:** standalone-mode
**Milestone:** M3 — Auth Evolution (roadmap entry: "Standalone Mode")
**Status:** Specified
**Created:** 2026-07-06
**Scope:** Complex (new persistence backend + cache + email + secrets + build/packaging; workspace-wide feature flags)

---

## 1. Objective

Provide a **compile-time `standalone` build** of the gateway that runs with **zero external
runtime services** — no PostgreSQL, no Redis, no SMTP server, no Vault. A single self-contained
binary that boots against a local SQLite file, an in-process cache, and a non-delivering email
transport, so a new user can go from `docker run` to a working gateway in under two minutes.

The existing **full** mode (PostgreSQL + Redis + SMTP + optional Vault) must remain **byte-for-byte
unchanged** — it is the default build.

### Target use cases

- Evaluation / onboarding (`docker run mycelium` with no compose file)
- Edge / field deployments (no infrastructure available)
- Air-gapped environments (biotech, agro)
- Single-instance deployments with no replication requirement

---

## 2. Verified current state (baseline)

Established by direct code inspection on 2026-07-06 (gateway at `8.3.1-rc.5`, Diesel `2.3.7`):

| Concern | Reality |
|---|---|
| ORM | Synchronous `diesel` 2.3.7 + `r2d2` pool (`Pool<ConnectionManager<PgConnection>>`). **Not** `diesel_async`. DB calls run blocking inside async handlers today (no `spawn_blocking`). |
| Hexagonal separation | Clean. `core/` has zero diesel/redis deps. Diesel adapter implements **~44 repository port traits** from `core/src/domain/entities/`. |
| KV port | Two traits only: `KVArtifactRead::get_encoded_artifact(key) -> String` and `KVArtifactWrite::set_encoded_artifact(key, value, ttl)`. Backed by Redis in `adapters/kv_db`. |
| Redis usage | (a) artifact caching — JWKS, user profiles, email profiles (TTL'd); (b) email **message queue** via `LPUSH` in `adapters/notifier/src/repositories/local_message_sending.rs`. |
| Email | `adapters/notifier` uses `SmtpTransport` only. `lettre = "0.11"` declared with **no features** — `file-transport` / `stub-transport` are not compiled in. |
| Config | `ConfigHandler` requires `core, diesel, api, auth, smtp, queue, redis`; only `vault` is `OptionalConfig`. **No `mode` field exists.** |
| Boot dependencies | **Postgres, Redis, and SMTP all hard-fail (panic) at boot** if unavailable (`ports/api/src/main.rs` `initialize_modules`). Vault is lazy/optional. |
| Secrets | `SecretResolver<T>` supports `Value` (plain), `Env`, `Vault`. Vault already optional (`OptionalConfig<VaultConfig>`). |
| Feature flags | Only one exists workspace-wide: `rhai` in `ports/api/Cargo.toml`. **No `standalone` / `sqlite` / `full` flags.** |
| Dockerfile | `rust:latest` base (builder + runtime), installs `libpq-dev`, official image builds `--features rhai`, entrypoint `myc-api`. |
| SQLite / moka | **Zero references** anywhere in the workspace. |

---

## 3. Corrections to the source brief

The originating brief contained claims that do **not** match the codebase. The spec corrects them;
downstream design and marketing must use the corrected version.

| # | Brief claim | Verified reality | Consequence |
|---|---|---|---|
| C-1 | "Pub-sub interno: Redis → `tokio::sync::broadcast`" | **No Redis pub/sub exists.** The only Redis-queue usage is an `LPUSH` email list queue. There are no `subscribe`/`publish` calls and no `tokio::sync::broadcast` today. | There is no pub/sub to port. The real item is the **email queue** (see SM-R5 / design §Email). The `broadcast` row is dropped. |
| C-2 | Limitation #1: "Sem session tracking distribuído — rate limiting e detecção de retry loop de AI agents não funcionam entre processos" | **Neither rate limiting nor AI-agent retry-loop detection exists** in the codebase. Rate limiting is a *future* item (roadmap M4). | Standalone loses **nothing that exists today**. This "limitation" is removed; replaced by a forward-looking note (L-1). |
| C-3 | Limitation #3: "tokens invalidados manualmente antes de restart podem reaparecer válidos" | Token invalidation is **DB-backed** (`TokenInvalidation` trait, diesel), not Redis. Redis only caches profiles/JWKS with TTL. | Overstated. In standalone the DB (SQLite) persists invalidation; only the *TTL'd profile cache* is lost on restart, which self-heals within the TTL. Corrected in L-3. |
| C-4 | Roadmap: "in-memory KV store replaces Redis (`mem_db` adapter already exists — wire it)" | **`mem_db` does not implement the KV traits.** It implements `RoutesRead` / `ServiceRead` / `ServiceWrite` (service-catalog cache), unrelated to `KVArtifact*`. | A **new `moka` adapter** implementing `KVArtifactRead`/`KVArtifactWrite` is required. The brief's moka proposal is correct; the roadmap line is wrong and will be updated. |
| C-5 | Market claim: "Single binary, single dependency (PostgreSQL)" | Today there are **three** hard boot dependencies: Postgres **+ Redis + SMTP**. | "Zero dependencies" is a real improvement, but the *current* baseline is 3 deps, not 1. Marketing copy must not claim the pre-standalone product had a single dependency. |
| C-6 | "Se o projeto usa `diesel_async`… backend SQLite é síncrono, use `spawn_blocking`" | Project is **already synchronous** diesel + r2d2. | The `diesel_async` caveat is moot. SQLite fits the existing sync pattern with no async-bridging change. |
| C-7 | "Diesel: feature `sqlite`; single runtime `mode` flag possible" | `MultiConnection` (Diesel 2.2+) is available but **cannot** span these tables: schema uses `Array<Text>`, `Array<Jsonb>`, `Jsonb`, `Uuid`, `Timestamptz` — **all Postgres-only** Diesel SQL types. A shared runtime-switchable schema will not compile for SQLite. | Backend selection is **compile-time** (`standalone` feature), producing a separate binary. This **contradicts the roadmap's `mode = "standalone"` runtime flag** — roadmap to be updated. See DEC-1. |

---

## 4. Decisions (resolved gray areas)

| ID | Decision | Rationale |
|---|---|---|
| DEC-1 | **Compile-time `standalone` cargo feature** selects SQLite + moka + stub/file email. Separate binary/image. Full mode is the default build and stays unchanged. | Forced by C-7: `Array`/`Jsonb`/`Uuid`/`Timestamptz` are Postgres-only in Diesel; a runtime switch would require storing those as TEXT in Postgres too, which changes full mode (violates the hard constraint). |
| DEC-2 | **Secrets auto-generated on first boot and persisted**, reused across restarts. **Prefer OS keyring** (`keyring` crate) when a secret service is available; **fall back to an encrypted local file** (0600) otherwise. | Zero-config onboarding. Persistence prevents envelope-encryption KEK / HMAC key from rotating on every restart (which would invalidate all connection strings — see STATE AD-004). **Open concern OC-2:** the primary targets (containers, air-gapped, edge) usually have *no* keyring daemon, so the file fallback is the de-facto primary path. |
| DEC-3 | **Email fallback = StubTransport by default** (log rendered message + magic-link URL to `tracing`), with **opt-in FileTransport** (`.eml` to a local dir) via config. Real SMTP still used if `[smtp]` is configured. Requires enabling lettre `file-transport` (+ `stub-transport`) features. | Onboarding needs the magic-link URL visible immediately (stdout); file transport serves operators who want the raw message. |
| DEC-4 | **Embedded migrations** via `diesel_migrations` (`embed_migrations!`) for the SQLite backend, so the binary auto-provisions the DB file on first boot. | Current migrations are manual `psql` SQL files — no auto-provisioning exists. Zero-infra onboarding requires the binary to create/migrate the DB itself. Full-mode Postgres migration workflow is left unchanged. |
| DEC-5 | Deliverable of this pass = **spec.md + design.md**. No implementation. | Complex feature; user requested mapping ("mapear como especificações"). |

---

## 5. Functional requirements

Each requirement has a traceable ID for design/tasks.

### Persistence (SQLite)

- **SM-R1** — When built with `--features standalone`, the gateway MUST persist all domain data to a
  local SQLite database file (path from config, default e.g. `./data/mycelium.db`), using Diesel's
  SQLite backend with `libsqlite3-sys` `bundled` (SQLite compiled into the binary; no system libsqlite).
- **SM-R2** — The SQLite adapter MUST implement the **same `core` port traits** as the Postgres
  adapter (~44 repository traits). `core/` and use-cases MUST NOT change.
- **SM-R3** — On first boot the standalone binary MUST auto-create and migrate the SQLite schema
  (embedded migrations). No external migration step.
- **SM-R4** — Postgres-specific column types MUST be mapped for SQLite per the documented table
  (design §Type mapping): `Uuid`→`TEXT`, `Jsonb`/`Array<*>`→`TEXT` (JSON-serialized),
  `Timestamptz`→`TEXT` ISO-8601 (UTC). Round-trip fidelity (write→read equals original) MUST hold.

### Cache (moka)

- **SM-R5** — A new in-process cache adapter (`moka`, `future` feature) MUST implement
  `KVArtifactRead` and `KVArtifactWrite`, honoring the per-key `ttl` argument. It replaces Redis in
  standalone. Cache is process-local and non-persistent (expected for single-instance).
- **SM-R6** — The email **queue** (Redis `LPUSH` today) MUST have a standalone replacement. Design
  will choose between an in-process queue (`tokio::sync::mpsc`) with an in-process consumer, or a
  direct synchronous send bypassing the queue. (See OC-1.)

### Email

- **SM-R7** — With no `[smtp]` configured, the gateway MUST NOT fail at boot (unlike today). It MUST
  route outbound mail to a non-delivering transport: **StubTransport** (default; logs subject, body,
  and any magic-link URL) or **FileTransport** (`.eml` to a configured directory) when selected.
- **SM-R8** — If `[smtp]` IS configured in a standalone build, real SMTP delivery MUST still work
  (opt back in). The fallback is only for the unconfigured case.

### Secrets / config

- **SM-R9** — Standalone MUST boot with **no Vault** and no operator-provided secrets: it
  auto-generates `token_secret`, the JWT/HMAC secret set, and any other required secrets on first
  boot and **persists them** (keyring-preferred, encrypted-file fallback per DEC-2), reusing them on
  subsequent boots.
- **SM-R10** — The standalone config MUST make `[diesel]`(→sqlite path), `[redis]`, `[smtp]`,
  `[queue]`, `[vault]` sections **optional/irrelevant**, replaced by a minimal standalone config
  surface. Full-mode config parsing MUST be unaffected.

### Build / packaging

- **SM-R11** — `cargo build` (default) MUST produce the unchanged full binary. `cargo build
  --no-default-features --features standalone` MUST produce the zero-dependency binary. Because Cargo
  features are additive, `standalone` requires `--no-default-features` (otherwise the default
  `postgres-backend` stays on and the mutual-exclusion `compile_error!` fires). The two feature sets
  are mutually exclusive (design §1).
- **SM-R12** — A standalone Docker image MUST build the `standalone` binary and MUST NOT install
  `libpq-dev` or any DB/Redis client system library. Ideally a minimal runtime base (e.g.
  `debian:slim` or `distroless`) carrying only the binary + templates + `ca-certificates`.
- **SM-R13** — `docker run <standalone-image>` with no configuration MUST yield a working gateway
  (health endpoint responds; a JWT can be issued; a route can be added) — the <2 min onboarding goal.

### Non-regression

- **SM-R14** — Full mode gate checks MUST pass unchanged: `cargo fmt --all -- --check`,
  `cargo build --workspace`, `cargo test --workspace --all`. No behavioral change to full mode.
- **SM-R15** — Standalone build MUST also pass fmt/build; standalone-specific tests MUST cover the
  SQLite type round-trips (SM-R4) and the moka TTL behavior (SM-R5).

---

## 6. Corrected limitations (must be documented before any "zero dependencies" marketing)

- **L-1** — No distributed session tracking. *(Forward-looking: when gateway-level rate limiting /
  AI-agent retry-loop detection ships in M4, those features will require shared state and will not
  work across processes in standalone. Neither exists today — standalone loses nothing currently.)*
- **L-2** — No replication / single instance only. Do not run standalone behind a load balancer with
  multiple replicas (SQLite file + in-process cache are node-local).
- **L-3** — Cache does not persist across restarts. Token **invalidation** is durable (SQLite), but
  the TTL'd **profile/JWKS cache** is cold after restart and re-populates on demand — no correctness
  loss, only a brief cache-miss window. *(Corrected from brief C-3.)*
- **L-4** — SQLite write concurrency: global write lock. Acceptable for single-instance /
  low-write-volume; a bottleneck under high concurrent writes. WAL mode recommended (design).
- **L-5** — Email not delivered in stub/file mode. Operators must read stdout (stub) or the `.eml`
  directory (file) to retrieve magic links. Must be stated in onboarding docs.
- **L-6** — Secrets file (fallback path) holds the KEK/HMAC material at rest; losing or leaking it
  loses/exposes all encrypted data and connection-string signing. Must be protected and backed up.

---

## 7. Out of scope

- Runtime switching between backends (`mode` flag) — rejected by DEC-1.
- Migrating existing Postgres data into SQLite (no data-migration tooling).
- High-availability / clustering for standalone.
- Changing full-mode schema, queries, or the manual Postgres migration workflow.
- Rate limiting / session tracking implementation (roadmap M4).

---

## 8. Open concerns (carry into design)

- **OC-1** — Email queue consumer: today `LPUSH` enqueues and *something* drains it (verify whether a
  separate worker/CLI or an in-process task consumes the Redis list). Standalone must either run an
  in-process consumer over a `tokio::sync::mpsc` queue or bypass the queue and send directly to the
  stub/file transport. Design must resolve and note the full-mode consumer location.
- **OC-2** — Keyring availability: the stated targets (containers, air-gapped, edge) typically lack
  an OS secret-service daemon, so keyring will usually be unavailable and the encrypted-file fallback
  becomes primary. Confirm the `keyring` crate degrades gracefully (does not panic) when no backend
  is present.
- **OC-3** — Effort magnitude: because `Uuid`, `Timestamptz`, `Jsonb`, and `Array` are *all*
  Postgres-only in Diesel, essentially **every** table needs a SQLite-specific schema + model, and
  all ~44 repository impls need SQLite variants. This is the dominant cost and risk — design must
  size it and decide code-sharing strategy (feature-gated modules vs sibling crate).
- **OC-4** — Query-level Postgres-isms (distinct axis from column types): the diesel repository bodies
  use Postgres-only *SQL operations* that TEXT column-mapping alone will not fix. Verified counts in
  `adapters/diesel_postgres/src`: `jsonb_set` ×1 (magic-link token invalidation, per STATE), JSON `->>`/`->`
  ×17, JSONB containment `@>` ×8, `diesel::pg`/`::pg::` ×17, raw `sql`/`sql_query` ×18. Each needs a
  SQLite equivalent (`json_set`, JSON1 `->>`, JSON1 functions, backend-agnostic DSL). This reinforces
  that the SQLite backend is close to a **second repository-layer implementation**, not a type swap.

---

## 9. Acceptance criteria

- [ ] `cargo build` (default/full) unchanged; full-mode gate checks pass (SM-R14).
- [ ] `cargo build --no-default-features --features standalone` produces a binary linking no system
      libpq/redis/libsqlite.
- [ ] CI builds **both** the default and `--no-default-features --features standalone` targets so the
      standalone feature cannot rot silently.
- [ ] Standalone binary boots with an empty working dir: auto-creates + migrates SQLite, auto-generates
      and persists secrets, starts with stub email — no Postgres/Redis/SMTP/Vault present.
- [ ] Issue a JWT (magic-link or email+password) and see the link in stdout (stub) end-to-end.
- [ ] Add a downstream route and proxy a request through the standalone gateway.
- [ ] SQLite type round-trip tests pass (SM-R4); moka TTL test passes (SM-R5).
- [ ] `docker run <standalone-image>` with zero config reaches a working gateway (SM-R13).
- [ ] All six brief corrections (C-1…C-7) reflected; roadmap + limitations docs updated (C-4, C-7).
