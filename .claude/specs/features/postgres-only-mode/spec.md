# Feature Spec: Postgres-Only Mode

**Feature:** postgres-only-mode
**Milestone:** (to slot into ROADMAP — deployment/packaging)
**Status:** Specified
**Created:** 2026-07-22
**Scope:** Complex (new KV cache backend + new email-queue substrate + config surface + DI wiring + packaging; workspace-wide feature flags)
**Branch:** `feat/postgres-only-mode`

---

## 1. Objective

Add a **third compile-time build mode** to the gateway — **Postgres-only** — that runs the full
production feature set (Postgres persistence, real SMTP email, **multi-pod horizontal scaling**) but
**without Redis**. Both jobs Redis does today move to Postgres:

1. the KV/artifact cache (`KVArtifact*` port), and
2. the notifier's email **queue** (Redis `LPUSH` list → a multi-consumer-safe Postgres queue).

The existing **full** mode (Postgres + Redis + SMTP) stays **byte-for-byte unchanged and remains the
Cargo `default` build and the published Docker image** (no breaking change). The **standalone** mode
(SQLite + moka + stub/file email) is **unchanged**. The `[redis]` config section becomes
unnecessary in Postgres-only and standalone builds, and stays required only in full.

### The three modes

| Mode | Persistence | KV / cache | Email queue | Email transport | Cargo build | Redis | Scale |
|---|---|---|---|---|---|---|---|
| **standalone** | SQLite | moka (in-process) | in-process message-table poll | stub/file (SMTP if configured) | `--no-default-features --features standalone` | none | single-instance |
| **full** *(default, unchanged)* | Postgres | Redis (`SETEX`) | Redis `LPUSH` list | SMTP | default (`full`) | **required** | multi-pod |
| **postgres-only** *(new, opt-in)* | Postgres | Postgres cache table | Postgres queue (`SKIP LOCKED`) | SMTP | opt-in feature flag | none | multi-pod |

### Target use cases

- Production deployments (K8s, multi-pod) that want to drop Redis as an infrastructure component
  while keeping Postgres, real email, and horizontal scaling.
- Environments where standalone's SQLite single-instance model is too limited but Redis is
  unwanted operational overhead.

---

## 2. Verified current state (baseline)

Established by direct code inspection on 2026-07-22 (gateway at `9.0.0-rc.6`).

| Concern | Reality |
|---|---|
| KV port | Exactly two `shaku::Interface` traits in `core/src/domain/entities/kv_artifact/`: `KVArtifactRead::get_encoded_artifact(key: String) -> FetchResponseKind<String,String>` and `KVArtifactWrite::set_encoded_artifact(key: String, value: String, ttl: u64) -> CreateResponseKind<String>`. No delete/scan/batch/pub-sub in the port. |
| Redis usage | Only **one live role**: KV cache in `adapters/kv_db` (`myc_kv`) via `SET`/`SETEX` (per-key TTL, seconds). The "email queue via `LPUSH`" is **vestigial dead code** (see next row). No Redis pub/sub anywhere (standalone spec C-1). |
| Email queue reality *(corrected 2026-07-22)* | The **real** email path in full mode is the Postgres **`message_queue` table** (`adapters/diesel_postgres/sql/up.sql:248`, columns `id, message, created, attempted, status, attempts, error`) written by `LocalMessageWrite` (SQL adapter) and polled by the in-process `email_dispatcher` (`ports/api/src/dispatchers/email_dispatcher.rs`), which sends via SMTP `RemoteMessageWrite` and `DELETE`s the row on success. The Redis `LPUSH` in `adapters/notifier/src/repositories/local_message_sending.rs` is **never consumed** (grep for `LPOP/RPOP/BRPOP/LRANGE` = 0) and **never produced from** (no producer resolves `LocalMessageWrite` from the notifier module — all use the SQL module). It is a dead binding, safe to drop. |
| Dispatcher concurrency gap *(pre-existing bug)* | The dispatcher does a plain `SELECT … WHERE status='Queued'` (`list_oldest_messages`, limit 25) and only `DELETE`s a row **after** the SMTP send returns. There is **no** `FOR UPDATE`, advisory lock, or claim/in-progress status. **Two pods in full mode already double-send email today.** Only the random startup jitter staggers them; it does not prevent steady-state overlap. |
| Message port traits | `LocalMessageReading::list_oldest_messages(tail_size: i32, status: MessageStatus)`; `LocalMessageWrite::{send, update_message_event, delete_message_event, ping}`; `RemoteMessageWrite::send(Message)` (SMTP). DTO `MessageSendingEvent { id, message, created, attempted, status, attempts, error }`; `MessageStatus { Queued, Sent, Failed }` (note: `Sent` is unused — success path deletes the row). |
| Alternative-KV precedent | `adapters/moka_cache` (`myc_moka_cache`, standalone) implements the same two KV traits over `moka::future::Cache<String,(String,Duration)>` with a `PerKeyExpiry` `Expiry` impl. Exposes a `KVAppModule` that drops into the same DI slot. This is the template a Postgres KV adapter follows. |
| Postgres pool reuse | `adapters/diesel_postgres` exposes `trait DbPoolProvider { fn get_pool() -> Pool<ConnectionManager<PgConnection>> }` (`models/config.rs:42`). A Postgres KV/cache adapter can inject `Arc<dyn DbPoolProvider>` to reuse the **same** pool — no second connection pool. |
| Postgres migrations | Raw SQL, operator-applied via `psql`. Base DDL `adapters/diesel_postgres/sql/up.sql`; incremental files in `adapters/diesel_postgres/sql/migrations/` named `YYYYMMDD_NN_<name>.sql` (not folded back into `up.sql`). `schema.rs` holds `diesel::table!` blocks. **No** `embed_migrations!` on the Postgres side (that exists only for SQLite/standalone). |
| DI indirection | `ports/api/src/models/active_backend_modules.rs` re-exports `SqlAppModule`/`KVAppModule` per `#[cfg(feature=…)]` so ~50 handler/middleware/dispatcher files stay backend-agnostic. Two cfg-gated `initialize_modules` fns (full vs standalone) build the concrete modules. |
| `[redis]` config load | `RedisConfig::from_default_config_file` (`adapters/shared/src/models/redis_config.rs`) deserializes the `[redis]` TOML section. In `ports/api/src/models/config_handler.rs` the `redis` field and its load call are `#[cfg(feature = "full")]` — a missing `[redis]` fails boot only in that build; standalone compiles it out entirely. `[queue]` config is backend-neutral and defaulted (loaded in both modes). |
| Backend features today | `default = ["full"]` (Postgres+Redis) and `standalone` (SQLite+moka), **mutually exclusive** via `compile_error!` in `main.rs`. Plus orthogonal `rhai`. |
| `mem_db` | `adapters/mem_db` (`MemDbAppModule`) is the routes/service-catalog cache — a **different** port from `KVArtifact*`. Built in both modes, backend-neutral. **Not affected** by this feature. |
| Persistence layer | Synchronous `diesel` + `r2d2` (`Pool<ConnectionManager<PgConnection>>`). Postgres migrations are manual SQL (`postgres/sql/up.sql` + `sql/migrations/`). |

---

## 3. Decisions (resolved gray areas)

Source: discuss phase — see `./context.md`.

| ID | Decision | Rationale |
|---|---|---|
| DEC-1 | **Multi-pod target.** Postgres-only mode must scale horizontally like full mode. The Redis email-queue coordination is replaced by a **multi-consumer-safe Postgres queue** (`SELECT … FOR UPDATE SKIP LOCKED` or advisory locks), not standalone's single-instance in-process poller. | It is a production replacement for full-mode-minus-Redis; production is K8s multi-pod. Concurrent pods must never double-send an email. |
| DEC-2 | **Cargo `default` build and the published image stay `full` (Postgres+Redis), unchanged.** Postgres-only is an **opt-in** feature/tag. | No breaking change to existing deployments pulling the default image. "default" in the original request was overloaded; the new mode is a third opt-in mode. |
| DEC-3 | **KV cache = a Postgres table** with an `expires_at` column; lazy expiry on read + periodic sweeper. Not moka, not two-tier. | Matches "KV no Postgres no lugar do Redis" literally; cross-pod shared + persistent. Accepted trade-off: cache reads/writes now hit Postgres. |
| DEC-4 | **Real SMTP email is retained** in Postgres-only mode (unchanged from full). Only the queue *substrate* changes (Redis list → Postgres table). | Postgres-only = full minus Redis, not standalone. Onboarding/stub email is a standalone concern only. |
| DEC-5 | **The Postgres KV adapter is its own sibling crate** under `adapters/` (mirroring `moka_cache`), never bolted into `kv_db`. | Adapter-crate-separation rule; screaming-architecture (one concept per crate). `core/` unchanged. |
| DEC-6 | **Exact Cargo feature graph is a Design decision.** The spec fixes only that: full and postgres-only **share** the Postgres/diesel persistence layer and **diverge at KV + email queue**; standalone is orthogonal. | Advisor guidance — keep feature-graph mechanics (three exclusive modes vs additive `redis` toggle on `full`) out of Specify. |
| DEC-7 | **Compile-time selection at the DI layer only** (`active_backend_modules.rs` + `initialize_modules`). `core/` and use-cases MUST NOT change. | Same invariant standalone established. |
| DEC-8 | **No new email queue is built.** Postgres-only reuses the existing `message_queue` table + `email_dispatcher` (already the real path in full mode). The multi-pod double-send gap is closed by adding `SELECT … FOR UPDATE SKIP LOCKED` claiming to the **shared** dispatcher read path — which also fixes the pre-existing full-mode bug. The dead Redis `LPUSH` binding is dropped. | Discuss phase: "corrigir o dispatcher compartilhado". Cheaper and correct; the Redis queue was never functional. Relaxes POM-20's "full byte-for-byte unchanged" to allow this one intentional correctness improvement to the shared dispatcher (see POM-20). |

---

## 4. Out of scope

| Item | Reason |
|---|---|
| Changing the Cargo `default` build or the published Docker image | DEC-2 — they stay `full`. |
| Auto-migrating existing full-mode deployments off Redis | Operators opt in explicitly; no data migration tooling in this pass. |
| Any change to standalone mode | Standalone is unchanged (only shares the "no `[redis]`" config relaxation). |
| Any change to full mode behavior | Must stay byte-for-byte identical. |
| Runtime mode switch (a `mode=` config field) | Selection is compile-time, like standalone (DEC-7). |
| A generic pluggable-cache abstraction beyond the two `KVArtifact*` traits | The port is already the abstraction; only a new adapter is needed. |
| Rate limiting / distributed session tracking | Do not exist today (standalone spec C-2). |
| Redis pub/sub replacement | No Redis pub/sub exists (standalone spec C-1). |

---

## 5. Functional requirements

### Persistence & Redis removal

- **POM-1** — When built with the Postgres-only feature, the gateway MUST use the existing
  Postgres/diesel persistence adapter (`myc_diesel`) **unchanged** — same schema, same ~44 repos.
  No SQLite.
- **POM-2** — A Postgres-only build MUST require **no Redis** at compile time or runtime: no
  `redis` client is constructed and no connection is attempted during boot or operation.

### KV cache on Postgres

- **POM-3** — A new sibling adapter crate (DEC-5) MUST implement `KVArtifactRead` and
  `KVArtifactWrite` backed by Postgres, exposing a `KVAppModule` that drops into the existing DI
  slot in `active_backend_modules.rs` (no `ports/api` handler changes).
- **POM-4** — `set_encoded_artifact(key, value, ttl)` MUST persist the key→value with an absolute
  expiry derived from `ttl` (seconds). Setting an existing key MUST upsert (last-write-wins).
- **POM-5** — `get_encoded_artifact(key)` MUST return `Found(value)` only when the row exists AND is
  not expired; an expired or absent key MUST return `NotFound(Some(key))` (lazy expiry on read).
- **POM-6** — Expired rows MUST be reclaimed by a periodic in-process sweeper so the cache table
  cannot grow unbounded; the sweep interval SHALL be configurable with a sane default.
- **POM-7** — The cache table MUST be provisioned through the Postgres migration workflow
  (`sql/up.sql` + `sql/migrations/`), consistent with how other tables are created.
- **POM-8** — The cache MUST be shared across pods (a single Postgres table), giving the same
  cross-instance cache coherence Redis provides in full mode.

### Email queue — reuse `message_queue` + make the shared dispatcher multi-pod safe (DEC-8)

- **POM-9** — Postgres-only mode MUST reuse the existing Postgres `message_queue` table and the
  existing `email_dispatcher` (no new queue table or queue adapter). The dead Redis `LPUSH`
  `LocalMessageWrite` binding MUST NOT be wired in this mode.
- **POM-10** — The dispatcher's claim of pending messages MUST be **multi-consumer-safe**:
  concurrent pods MUST use `SELECT … FOR UPDATE SKIP LOCKED` (or Postgres advisory locks) so each
  `Queued` message is processed by **at most one pod**. WHEN two pods poll simultaneously THEN each
  message SHALL be claimed by exactly one pod (no double-send under horizontal scale).
- **POM-11** — Claiming MUST transition a message out of the `Queued` selection window while
  in-flight (e.g. a `Processing` status / `attempted` stamp under the row lock), so a crashed or
  slow pod's message becomes **re-claimable** after a visibility timeout rather than being either
  lost or double-sent. Success still deletes the row; repeated failure still lands on `Failed`.
- **POM-12** — Email transport MUST remain **real SMTP**, identical to full mode (DEC-4). The
  Postgres-only notifier wiring provides SMTP `RemoteMessageWrite` **without** requiring a Redis
  client.
- **POM-13** — The claim/lock read path MUST be added to the **shared** dispatcher, fixing the
  pre-existing full-mode double-send bug as well (DEC-8). It MUST require an index supporting the
  `status`-filtered, `created`-ordered claim query (none exists today).

### Config surface

- **POM-14** — In a Postgres-only build the `[redis]` config section MUST be **optional/absent**:
  boot MUST NOT require it and MUST NOT fail when it is missing.
- **POM-15** — WHEN a `[redis]` section IS present in a Postgres-only build THEN the system SHALL
  ignore it gracefully (optionally a startup warning), never error.
- **POM-16** — Standalone builds MUST continue to need no `[redis]` (no regression), and **full**
  builds MUST continue to **require** `[redis]` exactly as today.
- **POM-17** — A shipped `settings/config.postgres-only.example.toml` MUST document the minimal
  surface (Postgres, SMTP, cache/queue tuning; **no `[redis]`**).

### Wiring, packaging & non-regression

- **POM-18** — Mode selection MUST be compile-time at the DI layer only (`active_backend_modules.rs`
  + a cfg-gated `initialize_modules` path). `core/` and use-cases MUST NOT change (DEC-7).
- **POM-19** — Illegal feature combinations MUST be prevented with a clear `compile_error!` (exact
  feature graph per DEC-6): standalone is mutually exclusive with any Postgres backend; full and
  postgres-only select different KV+queue wiring and MUST NOT both be active.
- **POM-20** — Full mode and standalone mode MUST remain behaviorally unchanged, **with one
  intentional exception (DEC-8):** the shared `email_dispatcher` gains `SKIP LOCKED` claiming, which
  changes full mode's dispatch from "double-send under multi-pod" to "at-most-one-pod delivery".
  This is a correctness fix, not a regression; no other full-mode or standalone behavior changes,
  and the default build + published image stay full mode.
- **POM-21** — A **build path for the Postgres-only image** MUST exist (Dockerfile build-arg or a
  dedicated Dockerfile), WITHOUT changing the default published image. Whether CI auto-publishes a
  separate tag is deferred to Design/Tasks; the minimum is a documented, reproducible build.

---

## 6. User Stories

### P1: Run production on Postgres only (no Redis) ⭐ MVP

**User Story**: As an operator, I want to run the gateway in a multi-pod deployment backed only by
Postgres — no Redis — so I can remove Redis from my infrastructure while keeping real email and
horizontal scaling.

**Why P1**: This is the entire point of the feature; every requirement in §5 (except packaging
polish) serves it.

**Acceptance Criteria**:
1. WHEN the gateway is built with the Postgres-only feature and started with a config that has
   Postgres + SMTP and **no `[redis]`** THEN it SHALL boot successfully.
2. WHEN a cache write happens with a TTL and is read back before expiry THEN it SHALL return the
   stored value; WHEN read after expiry THEN it SHALL return NotFound.
3. WHEN an email is enqueued and multiple pods are running THEN exactly one pod SHALL deliver it via
   SMTP (no duplicate delivery).
4. WHEN a pod crashes after claiming an email but before sending THEN the message SHALL be
   re-claimed and delivered by another pod.

**Independent Test**: Build with the feature; run against a Postgres + a fake SMTP sink with two
gateway instances; issue an auth flow that triggers a cached JWKS/profile and an email; observe one
delivery and cache hits.

---

### P2: Packaging, example config & docs

**User Story**: As an operator, I want an example config and a reproducible image build for the
Postgres-only mode so I can deploy it without reverse-engineering feature flags.

**Why P2**: Needed for adoption, not for the mode to function.

**Acceptance Criteria**:
1. WHEN I read `settings/config.postgres-only.example.toml` THEN it SHALL show a working minimal
   config with no `[redis]`.
2. WHEN I follow the documented build command/Dockerfile THEN I SHALL get a Postgres-only image,
   and the default published image SHALL remain full mode.

**Independent Test**: `docker build` the documented path; run it against Postgres; hit `/health`.

---

### P3: Cache & queue observability

**User Story**: As an operator, I want metrics for cache hit/miss, cache-table size / sweep, and
queue depth / claim latency so I can monitor the Postgres-backed replacements.

**Why P3**: Operability nicety; the mode works without it.

**Acceptance Criteria**:
1. WHEN the sweeper runs THEN it SHALL emit a count of reclaimed rows (log/metric).
2. WHEN the queue is polled THEN queue depth SHALL be observable.

---

## 7. Edge Cases

- WHEN two pods `set` the same cache key concurrently THEN the table SHALL upsert (last write wins),
  no error.
- WHEN a cache row is expired but not yet swept THEN a read SHALL treat it as NotFound.
- WHEN the sweeper is delayed/fails once THEN correctness SHALL be unaffected (lazy expiry on read
  is the source of truth; the sweep only reclaims space).
- WHEN two pods dequeue simultaneously THEN `SKIP LOCKED` SHALL ensure only one claims each message.
- WHEN a claimed message's pod dies THEN after the visibility timeout the message SHALL be
  re-claimable; delivery is **at-least-once** — document that SMTP delivery is not transactional, so
  a crash between "SMTP accepted" and "mark sent" can re-send (acceptable; note idempotency limits).
- WHEN `[redis]` is present in a Postgres-only build THEN it SHALL be ignored, not fatal.
- WHEN a very large value is cached THEN it SHALL be stored in a `TEXT`/`BYTEA` column; document
  practical size expectations.
- WHEN the Postgres-only build is compiled together with `standalone` or `full` wiring THEN a
  `compile_error!` SHALL fire.

---

## 8. Requirement Traceability

| Requirement ID | Story | Phase | Status |
|---|---|---|---|
| POM-1 | P1 | Design | Pending |
| POM-2 | P1 | Design | Pending |
| POM-3 | P1 | Design | Pending |
| POM-4 | P1 | Design | Pending |
| POM-5 | P1 | Design | Pending |
| POM-6 | P1 | Design | Pending |
| POM-7 | P1 | Design | Pending |
| POM-8 | P1 | Design | Pending |
| POM-9 | P1 | Design | Pending |
| POM-10 | P1 | Design | Pending |
| POM-11 | P1 | Design | Pending |
| POM-12 | P1 | Design | Pending |
| POM-13 | P1 | Design | Pending |
| POM-14 | P1 | Design | Pending |
| POM-15 | P1 | Design | Pending |
| POM-16 | P1 | Design | Pending |
| POM-17 | P2 | Design | Pending |
| POM-18 | P1 | Design | Pending |
| POM-19 | P1 | Design | Pending |
| POM-20 | P1 | Design | Pending |
| POM-21 | P2 | Design | Pending |

**Coverage:** 21 total, 0 mapped to tasks yet, design pending.

---

## 9. Success Criteria

- [ ] A Postgres-only build boots and serves traffic against Postgres + SMTP with **no Redis** and
      no `[redis]` config.
- [ ] Two concurrent instances deliver each queued email exactly once (no double-send) and re-claim
      after a simulated crash.
- [ ] Cache round-trip + TTL expiry behave correctly; the cache table does not grow unbounded.
- [ ] `cargo build` (no flags) and the published Docker image are **still full mode**, byte-for-byte
      unchanged; standalone unchanged.
- [ ] `cargo build`, `cargo test --workspace`, and `cargo fmt --check` pass for all three modes.
