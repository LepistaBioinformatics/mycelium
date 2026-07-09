# Design: Standalone Mode

**Feature:** standalone-mode
**Spec:** `./spec.md`
**Status:** Design
**Created:** 2026-07-06

This design realizes the requirements in `spec.md`. It assumes DEC-1…DEC-5 and the corrections
C-1…C-7. The guiding invariant: **`core/` and use-cases do not change; full mode is the default and
is untouched; standalone is a compile-time feature that swaps adapter implementations.**

---

## 1. Architecture overview

```
                         core/ (domain + use_cases + port traits)   ← UNCHANGED
                                     ▲            ▲
                 ┌───────────────────┘            └───────────────────┐
        full (default)                                          standalone (feature)
   ┌───────────────────────┐                            ┌───────────────────────────┐
   │ diesel (Postgres)     │  persistence               │ diesel (SQLite backend)   │
   │ kv_db (Redis)         │  cache                      │ moka (in-process)         │
   │ notifier (SMTP + queue│  email                      │ notifier (stub/file)      │
   │        via Redis LPUSH)│                            │                           │
   │ config (Vault/env/plain)│ secrets                   │ config (autogen+keyring/  │
   └───────────────────────┘                            │           file fallback)  │
                                                         └───────────────────────────┘
        ▲                                                            ▲
        └──────────────── ports/api  (main.rs wires via #[cfg]) ─────┘
```

Selection is done with `#[cfg(feature = "standalone")]` / `#[cfg(not(feature = "standalone"))]`
at the **DI wiring layer only** (`ports/api/src/main.rs` `initialize_modules`). No use-case or
domain code is gated.

### Feature model (SM-R11)

```toml
# ports/api/Cargo.toml
[features]
default   = ["postgres-backend"]      # full mode
postgres-backend = ["myc-diesel/postgres", "dep:redis-adapter", ...]
standalone       = ["myc-diesel/sqlite", "dep:moka-adapter", "notifier/local-transport", ...]
```

`standalone` and `postgres-backend` are **mutually exclusive** — building with both is a hard error
(`compile_error!` guard in `main.rs`). Rationale: the two select different Diesel backends and
different `ConnectionManager` types that cannot coexist in one wiring path cleanly, and there is no
product need to ship both in one binary (DEC-1).

Feature propagation crosses crates. Each adapter crate exposes its own feature that `ports/api`
turns on:

| Crate | Full feature | Standalone feature | Effect |
|---|---|---|---|
| `myc-diesel` | `postgres` (default) | `sqlite` | selects backend + schema/model module + embedded migrations |
| new `myc-cache` (or feature on kv_db) | `redis` | `moka` | selects `KVArtifact*` impl |
| `myc-notifier` | `smtp` (default) | `local-transport` | enables lettre `file-transport`/`stub-transport`, selects transport |
| `mycelium-config` | — | `standalone-secrets` | enables autogen + keyring/file secret source |

---

## 2. Persistence — SQLite via Diesel (SM-R1…R4)

### 2.1 Why not `MultiConnection` / runtime switch

Diesel 2.3.7 offers `#[derive(MultiConnection)]`, but the schema declares Postgres-only SQL types on
essentially every table:

- `Uuid` (all PKs) — Postgres-only
- `Timestamptz` (created/updated everywhere) — Postgres-only
- `Jsonb` (meta, account_type, mfa, secret, propagations, headers, …) — Postgres-only
- `Array<Text>` (`guest_user_on_account.permit_flags` / `deny_flags`) — Postgres-only
- `Array<Jsonb>` (`tenant.status`) — Postgres-only

None of these compile against Diesel's SQLite backend. Therefore SQLite needs a **parallel
schema.rs and parallel model structs**, selected by feature. This is the dominant effort (OC-3).

### 2.2 Module layout (feature-gated within `myc-diesel`)

Keep a single crate; gate backend-specific modules:

```
adapters/diesel_postgres/src/
  schema.rs                     ← #[cfg(feature="postgres")]  (existing, unchanged)
  schema_sqlite.rs              ← #[cfg(feature="sqlite")]     (new: TEXT-based types)
  models/…                      ← split or cfg-gated per backend where types differ
  repositories/…                ← trait impls; cfg-gate the query/type-specific bodies
  migrations_sqlite/            ← embedded via diesel_migrations (new)
```

Repository trait *signatures* are identical (they speak `core` DTOs, SM-R2). Only the internal
Diesel query building and row mapping differ. Where a repository's body is type-agnostic it can be
shared; where it touches Uuid/Json/Array/timestamp columns it needs a SQLite variant. Expect the
majority of the ~44 impls to need a SQLite body (OC-3).

**Two independent portability axes (do not conflate):**
1. *Column types* — §2.3 maps `Uuid`/`Jsonb`/`Array`/`Timestamptz` → `TEXT`.
2. *Query operations* — the bodies also use Postgres-only SQL that TEXT-typing does **not** fix
   (OC-4). Verified in `adapters/diesel_postgres/src`: `jsonb_set` ×1 (magic-link invalidation → SQLite
   `json_set`), JSON `->>`/`->` ×17, JSONB `@>` ×8, `diesel::pg`/`::pg::` ×17, raw `sql`/`sql_query`
   ×18. Each site needs a SQLite (JSON1) equivalent or a backend-agnostic rewrite. This is why the
   SQLite backend is closer to a second repository-layer implementation than a type swap.

### 2.3 Type mapping (SM-R4)

| Postgres (Diesel) | SQLite storage | Encode | Decode |
|---|---|---|---|
| `Uuid` | `TEXT` | `uuid.to_string()` (hyphenated) | `Uuid::parse_str` |
| `Timestamptz` | `TEXT` (ISO-8601 UTC, `RFC3339`) | `dt.to_rfc3339()` | `DateTime::parse_from_rfc3339` → `Utc` |
| `Jsonb` | `TEXT` | `serde_json::to_string` | `serde_json::from_str` |
| `Array<Text>` | `TEXT` | JSON array string | JSON parse → `Vec<String>` |
| `Array<Jsonb>` | `TEXT` | JSON array string | JSON parse → `Vec<Value>` |
| `Nullable<T>` | `NULL`able TEXT | `Option` map | `Option` map |

Round-trip fidelity (write→read equals original) is a required test (SM-R15). Timestamps normalize
to UTC — document that sub-second/timezone-offset info is stored as UTC ISO strings.

### 2.4 Connection & pool

Reuse the existing sync + r2d2 pattern (C-6): `Pool<ConnectionManager<SqliteConnection>>`. Enable
`libsqlite3-sys` with `bundled` (SQLite compiled in; satisfies SM-R1 "no system lib" and SM-R12).
Set pragmatic pragmas on connect via a Diesel `CustomizeConnection`:

- `PRAGMA journal_mode = WAL;` (mitigates L-4 write-lock stalls)
- `PRAGMA foreign_keys = ON;`
- `PRAGMA busy_timeout = 5000;`

SQLite write concurrency is single-writer; keep the r2d2 pool small (or 1 writer + N readers). Note
DB calls still block the async worker (same as full mode today) — acceptable for single-instance.

### 2.5 Auto-provisioning (SM-R3, DEC-4)

Add `diesel_migrations` to the SQLite feature. On boot: if the DB file is absent, create it, then run
`embed_migrations!("migrations_sqlite")` before building the pool provider. Full-mode Postgres
migration workflow (manual `psql` of `sql/up.sql` + `sql/migrations/`) is left untouched.

---

## 3. Cache — moka (SM-R5)

New adapter satisfying the two KV traits (verified interface):

```rust
// impl KVArtifactRead / KVArtifactWrite over moka::future::Cache<String, (String /*value*/, expiry)>
async fn set_encoded_artifact(&self, key, value, ttl: u64) // insert with per-entry TTL
async fn get_encoded_artifact(&self, key) -> FetchResponseKind<String, String>
```

- Crate: `moka = { version = "*", features = ["future"] }`.
- Per-entry TTL: moka supports `expire_after` via an `Expiry` impl keyed on the entry's `ttl`
  (moka's global `time_to_live` is uniform; a per-key TTL needs the `Expiry` trait). Design note:
  implement `Expiry` returning the stored `ttl` so SM-R5's per-key semantics hold.
- Location: new crate `adapters/cache` (`myc-cache`) with `redis`/`moka` features, OR a `moka`
  feature added to `adapters/kv_db`. **Recommendation:** new `adapters/cache` crate that houses both
  the Redis and moka impls behind features, so `kv_db` isn't overloaded and the screaming-architecture
  rule is respected (one concept per module). Either way `core` is unchanged.
- Non-persistent across restarts — matches single-instance expectation (L-3). Correctness preserved
  because token invalidation is DB-backed (C-3).

---

## 4. Email — stub/file transport (SM-R7, R8; resolves OC-1)

### Existing pipeline (verified)

- `LocalMessageWrite` persists a message event; `email_dispatcher` (in `ports/api/src/dispatchers/`)
  is an **in-process `tokio::spawn` background task** that polls on an interval, reads pending
  messages via `LocalMessageReading` (resolved from the **SQL module**), and sends them via
  `RemoteMessageWrite`. The Redis `LPUSH` in `notifier`'s `local_message_sending.rs` is a secondary
  queue path used for multi-container coordination (jitter comment confirms).

### Standalone approach

- **No new consumer needed** (OC-1 resolved): reuse `email_dispatcher` as-is. Message persistence
  goes to the SQLite `message` table (diesel `LocalMessageReading`/`LocalMessageWrite` are already
  among the ~44 port traits, covered by §2). The Redis LPUSH path is simply not wired in standalone.
- **`RemoteMessageWrite`** resolves to a new local transport impl:
  - `StubTransport` (default): render the message and `tracing::info!` the subject, recipient, and
    any magic-link URL to stdout. (SM-R7)
  - `FileTransport` (opt-in): write `.eml` to a configured directory.
- lettre features: add `file-transport` and `stub-transport` to the `notifier` crate under its
  `local-transport` feature (workspace `lettre` stays featureless for full mode; the feature is
  additive and only compiled in standalone).
- If `[smtp]` is present in a standalone build, keep resolving the real `SmtpTransport` (SM-R8) —
  transport choice is: SMTP if configured → else file if `transport="file"` → else stub.

---

## 5. Secrets — autogen + keyring/file (SM-R9, DEC-2)

Standalone needs (at minimum) `token_secret` (envelope-encryption KEK + HMAC key derivation, per
STATE AD-003/AD-004) and the JWT/HMAC signing secret set.

**Resolution order on boot:**
1. If an explicit secret is provided (env / config `Value`), use it (SecretResolver already supports
   this — no change).
2. Else, load persisted standalone secrets from the **OS keyring** (`keyring` crate) if a backend is
   available.
3. Else, load from an **encrypted local secrets file** (0600) next to the SQLite DB.
4. Else (first boot, none found), **generate** cryptographically-random secrets, persist them to
   keyring (if available) or the file, and continue.

**OC-2 note:** the primary targets (containers, air-gapped, edge) usually have no Secret Service /
Keychain daemon, so step 2 will typically fail and step 3 (file) is the de-facto primary path. The
`keyring` call MUST degrade gracefully (catch the "no backend" error, fall through to file) and never
panic. The file path is what must be documented for backup/protection (L-6).

**Persistence is critical:** regenerating `token_secret` on every restart would rotate the KEK and
invalidate every connection string (STATE AD-004) — hence secrets are generated **once** and reused.

`[vault]` is already `OptionalConfig`, so no Vault present is fine today; standalone just adds the
autogen source. Full-mode secret resolution is unchanged.

---

## 6. Config surface (SM-R10)

`ConfigHandler` currently requires `core, diesel, api, auth, smtp, queue, redis`. For standalone we
avoid a runtime `mode` field (DEC-1) and instead make the config **shape** depend on the feature:

- Under `#[cfg(feature="standalone")]`, `ConfigHandler` (or a standalone variant) makes `redis`,
  `smtp`, `queue`, `vault` **optional/absent**, and reinterprets `[diesel]` as a SQLite file path
  (or introduces `[sqlite] path = "..."`).
- A shipped `settings/config.standalone.example.toml` documents the minimal surface (mostly `[api]`,
  optional `[sqlite]`, optional `[email]` transport selection, optional `[smtp]`).
- Full-mode `ConfigHandler` parsing is byte-identical (SM-R14).

Boot must **not** hard-fail on missing Redis/SMTP/Vault in standalone (contrast with current
`panic!`s in `initialize_modules`) — those code paths are `#[cfg]`-excluded in standalone.

---

## 7. DI wiring (`ports/api/src/main.rs`)

`initialize_modules` is the single integration point. Introduce two cfg-gated bodies:

```rust
#[cfg(all(feature = "standalone", feature = "postgres-backend"))]
compile_error!("features `standalone` and `postgres-backend` are mutually exclusive");

#[cfg(not(feature = "standalone"))]
async fn initialize_modules(...) { /* existing: diesel(pg) + redis + smtp + vault */ }

#[cfg(feature = "standalone")]
async fn initialize_modules(...) {
    // sqlite pool (autoprovision+migrate) → SqlAppModule
    // moka cache → KVAppModule
    // stub/file notifier → NotifierModule
    // autogen secrets
    // email_dispatcher spawned as today (reads from sqlite)
}
```

The `SqlAppModule`, `KVAppModule`, `NotifierModule` shaku module *types* stay the same; only the
concrete components registered differ. Downstream (`mem_db` service catalog, MCP, router) is
identical in both modes.

---

## 8. Packaging (SM-R12, R13)

New `Dockerfile.standalone` (or a build-arg on the existing Dockerfile):

- Builder: `rust:latest`, `cargo build --release --no-default-features --features standalone` (Cargo
  features are additive, so `--no-default-features` is required or the default `postgres-backend`
  stays on and trips the mutual-exclusion `compile_error!`; add `rhai` explicitly if wanted).
  `libsqlite3-sys` `bundled` means the builder needs a C toolchain (present in `rust:latest`) but the
  **runtime** needs no libsqlite.
- Runtime: minimal base (`debian:bookworm-slim` or `gcr.io/distroless/cc`) with only: the `myc-api`
  binary, templates dir, `ca-certificates`. **No `libpq-dev`.**
- Default a writable `/data` volume for the SQLite DB + secrets file.
- `docker run` with no config must satisfy SM-R13 (health + issue JWT + add route).

---

## 9. Effort / risk sizing (OC-3)

| Area | Effort | Risk |
|---|---|---|
| SQLite schema + models + ~44 repo impls (type mapping) | **High** (bulk of the work) | High — every table touches a pg-only type; parallel model set; test each round-trip |
| moka cache adapter | Low | Low — 2 tiny traits; only per-key TTL via `Expiry` needs care |
| Stub/file email + reuse dispatcher | Low | Low — dispatcher already in-process |
| Autogen secrets + keyring/file | Medium | Medium — keyring absent on targets (OC-2); must not break KEK stability |
| Workspace feature plumbing + cfg wiring | Medium | Medium — cross-crate feature propagation, mutual-exclusion guard, keep full mode identical |
| Standalone Dockerfile / minimal image | Low | Low |

**Primary risk:** the SQLite persistence layer (§2) is far larger than the brief implies; it is
close to a second full implementation of the repository layer. Recommend the Tasks phase break §2
into per-entity task groups and land them incrementally behind the feature flag, keeping full mode
green throughout.

---

## 10. Component breakdown (for the Tasks phase — not atomic yet)

1. **Feature scaffolding** — add `standalone`/`postgres-backend` features across crates + mutual-
   exclusion `compile_error!`; no behavior change yet.
2. **SQLite backend** — `schema_sqlite.rs`, model set, type-mapping helpers, embedded migrations,
   pool provider, per-entity repository impls (grouped: account, tenant, user, token, guest, message,
   webhook, error_code, licensed_resource, encryption_key, tags).
3. **moka cache adapter** — new `adapters/cache` crate (or feature) + `Expiry` TTL + tests.
4. **Local email transport** — lettre `file`/`stub` features; stub + file `RemoteMessageWrite` impls;
   transport selection logic.
5. **Autogen secrets** — keyring/file source, generate-on-first-boot, persistence, graceful keyring
   fallback.
6. **Config + wiring** — standalone config shape, cfg-gated `initialize_modules`, no hard-fail on
   missing services.
7. **Packaging** — `Dockerfile.standalone`, minimal runtime base, `/data` volume, zero-config run.
8. **Docs update** — correct roadmap (C-4, C-7), publish limitations L-1…L-6 before any "zero
   dependencies" marketing.

---

## 11. Traceability

| Requirement | Design section |
|---|---|
| SM-R1..R4 (SQLite) | §2 |
| SM-R5 (moka) | §3 |
| SM-R6 (queue) | §4 (OC-1 resolved) |
| SM-R7,R8 (email) | §4 |
| SM-R9,R10 (secrets/config) | §5, §6 |
| SM-R11 (features) | §1, §7 |
| SM-R12,R13 (docker) | §8 |
| SM-R14,R15 (non-regression/tests) | §7 (cfg isolation), §2.3, §3 |
