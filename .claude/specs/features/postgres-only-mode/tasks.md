# Postgres-Only Mode — Tasks

**Design:** `./design.md`
**Spec:** `./spec.md`
**Status:** Implemented — gates green, awaiting user validation (not committed)
**Branch:** `feat/postgres-only-mode`

## Verification results (T12) — 2026-07-22

- `cargo fmt --all -- --check` — clean.
- `cargo build --workspace` (default = full) — OK (no new warnings).
- `cargo build -p mycelium-api --no-default-features --features postgres-only,rhai` — OK.
- `cargo build -p mycelium-api --no-default-features --features standalone` — OK (no regression).
- `cargo test -p mycelium-postgres-kv` — 2 passed (pure `expires_at` logic).
- `cargo test -p mycelium-api --features postgres-only,rhai config_postgres_only` — 1 passed (example config parses, no `[redis]`).
- One pre-existing feature-gap fixed: `resource_audit_log_dispatcher` had no `postgres-only` arm → widened its `postgres-backend` cfg to `any(postgres-backend, postgres-only)`.

### Post-review refinement (advisor): claim tuning made configurable
- The claim's `visibility_timeout` and batch size were reviewed: reusing one 300s const coupled claim-protection with retry-backoff, silently raising full-mode failed-email retry latency to 300s, and lettre's ~60s default SMTP send made `10×60 > 300` re-introduce double-send. **User chose "make it configurable."**
- Added `[queue] claimBatchSize` (default 3) and `visibilityTimeoutSecs` (default 240) to `QueueConfig`. `claimBatchSize` flows through `consume_messages`/`email_dispatcher` into `list_oldest_messages`; `visibilityTimeoutSecs` is seeded into the diesel `LocalMessageReadSqlDbRepository` via shaku `with_component_parameters` in both Postgres `initialize_modules`. Defaults satisfy the invariant `window > batch × ~60s`. Exposed the read repo type (`pub use` in diesel `message` module) for seeding.
- Re-verified: fmt clean; full + postgres-only + standalone build; notifier 4, diesel 4, postgres_kv 2, postgres-only config-parse 1 — all pass.
- **Not run (need live Postgres + a real SMTP sink; deferred to user manual validation):** the two-pod dedup demo (POM-10/11) and cache round-trip/TTL against a real DB (POM-4/5). Per `commit-validation.md`, no commit until the user tests and approves.

---


> **Project rule override:** `commit-validation.md` forbids committing until the user manually
> tests and approves. The skill's "one commit per task" step is therefore **deferred** — all tasks
> are implemented, gates run, then the user validates before any commit. No task commits.
>
> **Parallelism model:** `[P]` tasks are dispatched as concurrent sub-agents that **only write
> files** (no `cargo`, no `git`) — the shared `target/` dir and single working tree make concurrent
> builds/VCS unsafe. All compilation + gate checks run **once, centrally**, in T12.

---

## Execution Plan

### Phase 1 — Foundation (sequential, orchestrator)
```
T1 → T2
```

### Phase 2 — Parallel authoring (isolated files, one sub-agent each)
```
T1 ─┬→ T3 [P]  (postgres_kv crate)
    ├→ T4 [P]  (migrations SQL)
T2 ─┤
    ├→ T5 [P]  (claim query, diesel)
    ├→ T6 [P]  (batch size, notifier executor)
    ├→ T7 [P]  (redis-free notifier module)
    ├→ T8 [P]  (example config)
    └→ T9 [P]  (Dockerfile build-arg)
```

### Phase 3 — Integration + verification (sequential, orchestrator)
```
T3,T7,T10 → T11 → T12
T2 → T10
```

---

## Task Breakdown

### T1: Create `postgres_kv` crate skeleton + register in workspace
**What:** New crate `adapters/postgres_kv` (Cargo.toml, `src/lib.rs`, empty `config`/`repositories`
modules) registered in root `Cargo.toml` `[workspace.dependencies]`.
**Where:** `adapters/postgres_kv/**` (new), `Cargo.toml` (root, add dep line).
**Depends on:** None **Reuses:** `adapters/moka_cache/Cargo.toml` layout **Requirement:** POM-3, DEC-5
**Done when:**
- [ ] Crate name `mycelium-postgres-kv`, lib `myc_postgres_kv`; deps: `myc-core`, `mycelium-base`, `mycelium-diesel`, `diesel`, `shaku`, `async-trait`, `tokio`, `tracing`, `chrono`.
- [ ] Registered: `mycelium-postgres-kv = { version = "9.0.0-rc.6", path = "adapters/postgres_kv" }`.
**Tests:** none (skeleton) **Gate:** build (deferred to T12)

### T2: Feature graph + guards + DI indirection
**What:** Add `postgres-only` feature + optional dep in `ports/api/Cargo.toml`; extend
`compile_error!` mutual-exclusion guards in `main.rs`; add cfg arms in `active_backend_modules.rs`.
**Where:** `ports/api/Cargo.toml`, `ports/api/src/main.rs` (guards only, lines ~24-33),
`ports/api/src/models/active_backend_modules.rs`.
**Depends on:** T1 **Reuses:** existing standalone cfg pattern **Requirement:** POM-18, POM-19, DEC-6
**Done when:**
- [ ] `postgres-only = ["dep:mycelium-diesel", "dep:mycelium-postgres-kv"]` (+ notifier smtp-only feature ref once T7 names it).
- [ ] `mycelium-postgres-kv = { workspace = true, optional = true }` in `[dependencies]`.
- [ ] Guards: `compile_error!` for each illegal pair among {postgres-backend, postgres-only, standalone} and for none-selected; `SqlAppModule` gated `any(postgres-backend, postgres-only)`; `KVAppModule` gets a `postgres-only → myc_postgres_kv::repositories::KVAppModule` arm.
**Tests:** none (wiring) **Gate:** build (T12)

### T3: `postgres_kv` cache adapter (provider + repos + sweeper + KVAppModule) [P]
**What:** Full Postgres KV adapter mirroring `moka_cache`, reusing `DbPoolProvider`.
**Where:** `adapters/postgres_kv/src/{config.rs, repositories/{mod.rs,kv_artifact_read.rs,kv_artifact_write.rs}, schema.rs}`.
**Depends on:** T1 **Reuses:** `adapters/moka_cache/src/*`, `adapters/diesel_postgres/src/models/config.rs` (`DbPoolProvider`), `adapters/diesel_postgres/src/repositories/.../local_message_read.rs` (diesel query style) **Requirement:** POM-3..8
**Done when:**
- [ ] `config.rs`: a provider component injecting `Arc<dyn DbPoolProvider>` (reuse pool, no new pool) + a self-spawned `tokio` interval sweeper running `DELETE FROM kv_artifact WHERE expires_at <= now()`.
- [ ] Local `diesel::table! { kv_artifact (key) { key -> Text, value -> Text, expires_at -> Timestamptz } }`.
- [ ] `KVArtifactWrite::set_encoded_artifact`: upsert `INSERT … ON CONFLICT (key) DO UPDATE`, `expires_at = now() + ttl secs`; returns `CreateResponseKind::Created(value)`.
- [ ] `KVArtifactRead::get_encoded_artifact`: `SELECT value WHERE key=$1 AND expires_at > now()` → `Found`/`NotFound(Some(key))`.
- [ ] `repositories/mod.rs` exposes `module! { pub KVAppModule { components = [Provider, KVArtifactReadRepository, KVArtifactWriteRepository] } }`.
- [ ] `#[tracing::instrument(name=…, skip_all)]` on public async fns; `MappedErrors` factories; no `unwrap` in prod.
- [ ] Unit test(s) for the pure TTL→`expires_at` computation (no live DB needed). DB round-trip is integration (manual/T12 note).
**Tests:** unit (pure logic only) **Gate:** build (T12); `cargo build -p mycelium-postgres-kv` must compile
**Sub-agent note:** write files only; do NOT run cargo/git.

### T4: Migrations — `kv_artifact` table + claim indexes [P]
**What:** SQL migration files + fold into `up.sql`.
**Where:** `adapters/diesel_postgres/sql/migrations/<date>_01_kv_artifact_cache.sql`,
`…/<date>_02_message_queue_claim_index.sql`, and append the same DDL to `adapters/diesel_postgres/sql/up.sql`.
**Depends on:** None **Reuses:** existing migration file format `YYYYMMDD_NN_<name>.sql` **Requirement:** POM-7, §4/§5
**Done when:**
- [ ] `kv_artifact (key TEXT PK, value TEXT NOT NULL, expires_at TIMESTAMPTZ NOT NULL)` + `INDEX idx_kv_artifact_expires_at (expires_at)`.
- [ ] `INDEX idx_message_queue_claim ON message_queue (status, created)`.
- [ ] Both blocks appended to `up.sql` (fresh-install path), placed near related tables.
- [ ] Use date `20260722`; do NOT edit `schema.rs` (kv_artifact `table!` lives in the crate, T3).
**Tests:** none (SQL) **Gate:** build (T12) **Sub-agent note:** write files only.

### T5: Multi-pod claim query in the Postgres message repo [P]
**What:** Rewrite `LocalMessageReadSqlDbRepository::list_oldest_messages` to an atomic
`UPDATE … RETURNING` claim with `FOR UPDATE SKIP LOCKED`, preserving `created DESC` and
`status='Queued'`; add a documented `VISIBILITY_TIMEOUT_SECS` const (default 300).
**Where:** `adapters/diesel_postgres/src/repositories/.../local_message_read.rs`.
**Depends on:** None (T4 index is runtime-only) **Reuses:** the file's existing diesel/base64 mapping **Requirement:** POM-9..13, POM-20, DEC-8
**Done when:**
- [ ] Claim query per design §3.2 (raw `sql_query`/diesel): stamps `attempted=now()` on the claimed set only, `SKIP LOCKED`, `ORDER BY created DESC`, `LIMIT tail_size`, filter `attempted IS NULL OR attempted < now() - interval`.
- [ ] Empty claim returns `FetchManyResponseKind::Found(vec![])`; row mapping unchanged (`map_model_to_dto`).
- [ ] Port signature UNCHANGED; a `// SPEC_DEVIATION:` comment notes the read-named port now UPDATEs (design deviation note).
- [ ] Comment states the safety invariant (`timeout > batch × max_send + margin`).
**Tests:** none unit (needs live DB → integration/manual, T12) **Gate:** build (T12) **Sub-agent note:** files only.

### T6: Reduce dispatcher claim batch size [P]
**What:** Change the hard-coded `list_oldest_messages(25, …)` to a small default (10) so the §3.2
invariant holds with the 300s default timeout.
**Where:** `adapters/notifier/src/executor/mod.rs` (the `consume_messages` read call).
**Depends on:** None **Reuses:** — **Requirement:** POM-13, design §3.2
**Done when:**
- [ ] Literal `25` → `10` with a comment referencing the visibility-timeout invariant.
**Tests:** none **Gate:** build (T12) **Sub-agent note:** files only, single edit.

### T7: Redis-free notifier module (`PostgresNotifierAppModule`) [P]
**What:** A shaku module exposing SMTP `RemoteMessageWrite` with NO Redis client, behind a notifier
feature (e.g. `smtp-only`).
**Where:** `adapters/notifier/src/repositories/mod.rs` (or a new module file), `adapters/notifier/Cargo.toml` (feature), `adapters/notifier/src/lib.rs` if needed.
**Depends on:** None **Reuses:** `remote_message_sending.rs` (SMTP, already Redis-free), the existing `NotifierAppModule` declaration for shape **Requirement:** POM-12
**Done when:**
- [ ] `module! { pub PostgresNotifierAppModule { components = [ <SMTP RemoteMessageSendingRepository> + its SmtpClient provider ] } }` — no `SharedClientImpl`, no `LocalMessageSendingRepository` (dead LPUSH).
- [ ] Gated so full mode's `NotifierAppModule` is untouched; expose the module type for `ports/api` to build.
**Tests:** none (module wiring) **Gate:** build (T12) **Sub-agent note:** files only; do NOT touch executor/mod.rs (T6 owns it).

### T8: Example config `config.postgres-only.example.toml` [P]
**What:** Minimal working config for the new mode (Postgres + SMTP, **no `[redis]`**), with queue
visibility-timeout note.
**Where:** `settings/config.postgres-only.example.toml` (new).
**Depends on:** None **Reuses:** `settings/config.standalone.example.toml` + `config.dev.for-docker.toml` for section shapes **Requirement:** POM-17
**Done when:**
- [ ] Has `[diesel]`, `[api]`, `[auth]`, `[smtp]`, `[queue]` (with the new fields), **no `[redis]`**; header comment explains the mode + `--features postgres-only` build.
**Tests:** none **Gate:** build (T12) **Sub-agent note:** file only.

### T9: Dockerfile `CARGO_FEATURES` build-arg [P]
**What:** Parametrize the build features; default preserves current full-mode image byte-for-byte.
**Where:** `Dockerfile`.
**Depends on:** None **Reuses:** existing Dockerfile install step **Requirement:** POM-21, TD-5
**Done when:**
- [ ] `ARG CARGO_FEATURES="postgres-backend,rhai"`; install uses `--no-default-features --features "${CARGO_FEATURES}"`; default output identical to today (full+rhai).
- [ ] Comment shows the postgres-only build-arg invocation + the pre-publish source-build caveat.
**Tests:** none **Gate:** build (T12) **Sub-agent note:** file only.

### T10: Config surface gating in `config_handler.rs`
**What:** Widen Postgres+SMTP field cfg gates to `any(postgres-backend, postgres-only)`; keep
`redis` gated to `postgres-backend` only; add the two `QueueConfig` fields if config-driven (else
const in T5).
**Where:** `ports/api/src/models/config_handler.rs` (+ `adapters/notifier/src/models/queue_config.rs` only if config-driven).
**Depends on:** T2 **Reuses:** existing cfg-gated fields **Requirement:** POM-14..17
**Done when:**
- [ ] `[redis]` never parsed under `postgres-only`; `diesel`/`smtp`/`api`/`auth`/vault fields available under both PG modes; standalone gates unchanged.
- [ ] A `#[cfg(feature="postgres-only")]` parse test mirrors the existing full/standalone ones.
**Tests:** unit (config parse test) **Gate:** build (T12)

### T11: `main.rs` wiring — third `initialize_modules` + spawn seams
**What:** Add the `#[cfg(feature="postgres-only")]` `initialize_modules` (PG diesel + postgres_kv
KV + PostgresNotifierAppModule, **no SharedAppModule**), plus the cfg arms in the call-site
destructure, `app_data` registration, and the dispatcher spawn.
**Where:** `ports/api/src/main.rs`.
**Depends on:** T2, T3, T7, T10 **Reuses:** the postgres-backend + standalone `initialize_modules` bodies **Requirement:** POM-1,2,18
**Done when:**
- [ ] Third `initialize_modules` returns the standalone-shaped tuple (no `SharedAppModule`), KV built from `myc_postgres_kv` seeded with the diesel `DbPoolProvider`, notifier = `PostgresNotifierAppModule` (SMTP).
- [ ] Call-site (`~317-360`), `app_data` (`~561-571`), dispatcher spawn (`~412-433`) get `postgres-only` cfg arms; dispatcher binds `LocalMessageReading`/`LocalMessageWrite` from the SQL module (claiming query from T5).
**Tests:** none unit (integration/manual) **Gate:** build (T12)

### T12: Verification — all three modes green
**What:** Compile, format, and test all three feature configurations; reason through two-pod dedup.
**Where:** whole workspace. **Depends on:** T1–T11 **Requirement:** §9 success criteria
**Done when:**
- [ ] `cargo fmt --all -- --check` clean.
- [ ] `cargo build --workspace` (default = full) OK.
- [ ] `cargo build -p mycelium-api --no-default-features --features postgres-only,rhai` OK.
- [ ] `cargo build -p mycelium-api --no-default-features --features standalone` OK (no regression).
- [ ] `cargo test --workspace --all` green (count not reduced).
- [ ] `cargo build -p mycelium-postgres-kv` OK.
**Tests:** all **Gate:** full/build

---

## Validation tables

### Granularity
| Task | Scope | Status |
|---|---|---|
| T1 | crate skeleton + 1 dep line | ✅ |
| T2 | feature graph (3 files, cohesive) | ✅ |
| T3 | one crate (cohesive adapter) | ✅ |
| T4 | migration SQL | ✅ |
| T5 | one function rewrite | ✅ |
| T6 | one literal | ✅ |
| T7 | one module | ✅ |
| T8 | one file | ✅ |
| T9 | one file | ✅ |
| T10 | one file (+opt config) | ✅ |
| T11 | main.rs wiring (cohesive) | ✅ |
| T12 | verification | ✅ |

### Diagram ↔ Depends-on cross-check
| Task | Depends (body) | Diagram | Status |
|---|---|---|---|
| T2 | T1 | T1→T2 | ✅ |
| T3 | T1 | T1→T3 | ✅ |
| T4 | — | (root) | ✅ |
| T5 | — | (root) | ✅ |
| T6 | — | (root) | ✅ |
| T7 | — | (root) | ✅ |
| T8 | — | (root) | ✅ |
| T9 | — | (root) | ✅ |
| T10 | T2 | T2→T10 | ✅ |
| T11 | T2,T3,T7,T10 | →T11 | ✅ |
| T12 | T1–T11 | →T12 | ✅ |

### Parallel-safety
All `[P]` tasks (T3–T9) touch **disjoint files** and run **without cargo/git** (write-only), so no
`target/` or VCS contention. T5/T4 share the diesel crate but different files; T6/T7 share the
notifier crate but different files. Central files (Cargo.toml root/ports, main.rs, config_handler,
active_backend_modules) are touched only by orchestrator tasks (T1, T2, T10, T11). ✅
