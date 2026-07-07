# Tasks: Standalone Mode

**Feature:** standalone-mode · **Spec:** `./spec.md` · **Design:** `./design.md`
**Branch:** `feat/standalone-mode` (gateway submodule) — NEVER `develop`
**Status:** Ready for execution
**Created:** 2026-07-06

Invariant on every task: **full mode stays byte-identical** (SM-R14). Gate for all Rust tasks:
`cargo fmt --all -- --check` + `cargo build --workspace` + `cargo test --workspace --all`, and (from
G1 onward) also `cargo build --no-default-features --features standalone`.

`[P]` = parallelizable with siblings once dependencies are met.

Legend for each task: **What / Where / Depends on / Reuses / Done when / Tests**.

---

## G1 — Feature scaffolding & build guards

### SM-T1 — Workspace feature scheme + mutual-exclusion guard — ✅ Done (verified, pending commit)
- **What:** Add `postgres-backend` (default) and `standalone` features to `ports/api`; wire them to
  per-crate features (created in later tasks, initially no-op). Add
  `#[cfg(all(feature="standalone", feature="postgres-backend"))] compile_error!(...)` in `main.rs`.
- **Where:** `ports/api/Cargo.toml`, `ports/api/src/main.rs`.
- **Depends on:** none.
- **Reuses:** existing `rhai` feature pattern.
- **Done when:** `cargo build` (default) unchanged; `cargo build --no-default-features --features standalone`
  compiles (even if it produces the full behavior for now); building with both features errors with the guard message.
- **Tests:** compile-only (both feature configs build).
- **Result:** `default = ["postgres-backend"]`; empty no-op `postgres-backend`/`standalone` markers;
  two `compile_error!` guards (both-features + neither-feature). Verified: `cargo check -p mycelium-api`
  (default) OK · `--no-default-features --features standalone` OK · `--features standalone` fails with
  the guard message. `cargo fmt --check` clean.

### SM-T2 — CI builds both targets — ✅ Done (verified, pending commit)
- **What:** Add a CI step building `--no-default-features --features standalone` alongside the default.
- **Where:** `.github/workflows/*` (gateway).
- **Depends on:** SM-T1.
- **Done when:** CI matrix builds default + standalone; standalone cannot rot silently.
- **Tests:** CI green on a no-op standalone build.
- **Result:** Added `standalone-build` job to `.github/workflows/ci.yml`
  (`cargo build -p mycelium-api --no-default-features --features standalone`). Scoped to `-p mycelium-api`
  because `--no-default-features --workspace` would break crates relying on default features. YAML
  validated; command verified locally. Runs on PR to main/develop.

---

## G2 — SQLite persistence (dominant effort — OC-3/OC-4)

### SM-T3 — SQLite deps behind `sqlite` feature on `myc-diesel` — ✅ Done (verified, pending commit)
- **What:** Add `diesel` `sqlite` feature, `libsqlite3-sys` (`bundled`), `diesel_migrations`, gated by
  a `sqlite` feature on the crate. Keep `postgres` as default feature. No models yet.
- **Where:** `adapters/diesel/Cargo.toml`.
- **Depends on:** SM-T1.
- **Done when:** crate builds under `--features sqlite` and (default) `--features postgres` independently.
- **Tests:** compile-only both features.
- **Result:** `myc-diesel` features `default=["postgres"]`, `postgres=["diesel/postgres"]`,
  `sqlite=["diesel/sqlite","libsqlite3-sys/bundled","dep:diesel_migrations"]`. `diesel` now declares
  only backend-agnostic features; `libsqlite3-sys 0.30 (bundled)` + `diesel_migrations 2` added as
  optional. Existing schema/models/migration/repositories gated behind `#[cfg(feature="postgres")]`
  in `lib.rs` so sqlite-only compiles empty. **Verified via `cargo tree`:** sqlite build pulls
  `libsqlite3-sys` and NO `pq-sys` (no libpq); postgres build pulls `pq-sys`. Full-mode
  build/test/fmt all green.

### SM-T4 — SQLite pool provider + connection pragmas — ✅ Done (verified, pending commit)
- **What:** `Pool<ConnectionManager<SqliteConnection>>` provider mirroring `DieselDbPoolProvider`;
  `CustomizeConnection` setting `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5000`. cfg-gated.
- **Where:** `adapters/diesel/src/sqlite/config.rs` (new `sqlite/` module tree).
- **Depends on:** SM-T3.
- **Reuses:** existing r2d2 pattern (sync — C-6).
- **Done when:** provider builds a pool against a temp file; pragmas verified via a query.
- **Tests:** unit — open temp DB, assert `journal_mode=wal`.
- **Result:** `sqlite/config.rs` — `SqliteDbPool`, `SqliteDbPoolProvider` trait, `DieselSqliteDbPoolProvider`
  shaku Component, `SqlitePragmas` customizer (WAL/foreign_keys/busy_timeout). Test green.

### SM-T5 — `schema_sqlite.rs` + embedded migrations — ✅ Done (verified, pending commit)
- **What:** Author `schema_sqlite.rs` (all tables, TEXT-based per §2.3) and `migrations_sqlite/` DDL
  mirroring `sql/up.sql` with SQLite types; wire `embed_migrations!` to auto-provision on boot.
- **Where:** `adapters/diesel/src/sqlite/schema.rs`, `adapters/diesel/migrations_sqlite/`.
- **Depends on:** SM-T3.
- **Reuses:** `schema.rs` (authoritative, post-migrations) as the source of truth.
- **Done when:** running embedded migrations on an empty temp DB creates every table; `diesel` compiles
  queries against `schema_sqlite` for a smoke table.
- **Tests:** unit — migrate temp DB, assert table set matches Postgres schema.
- **Result:** `sqlite/schema.rs` mirrors all 18 diesel tables (Uuid/Jsonb/Array/Timestamptz → Text).
  `migrations_sqlite/2026-07-06-000000_init/{up,down}.sql` creates all tables + `licensed_resources`
  and `public_connection_string_info` views (pg JSON `->`/`?` rewritten to JSON1 `json_extract`).
  `sqlite/migration.rs` embeds them via `embed_migrations!` + `run_pending_migrations`. Test asserts
  all 18 tables created. **Note:** `telegram_identity_audit` (pg raw-SQL/INET path) deferred — not in
  diesel `schema.rs`; standalone Telegram audit is a later concern.

### SM-T6 — Type-mapping helpers + round-trip tests (SM-R4) — ✅ Done (verified, pending commit)
- **What:** Encode/decode helpers: `Uuid↔TEXT`, `Timestamptz↔RFC3339 UTC`, `Jsonb↔TEXT`,
  `Array<Text>↔JSON`, `Array<Jsonb>↔JSON`. Central module reused by all sqlite repos.
- **Where:** `adapters/diesel/src/sqlite/types.rs` (new).
- **Depends on:** SM-T5.
- **Done when:** each helper round-trips (write→read == original), incl. NULL and empty-array cases.
- **Tests:** unit — property/round-trip per type (the SM-R15 requirement).
- **Result:** `sqlite/types.rs` — `{uuid,timestamp,json,string_array,json_array}_{to,from}_text`,
  errors via `dto_err`. 6 round-trip tests (incl. empty array, garbage-UUID rejection, UTC
  normalization). All green.

### SM-T7..T17 — SQLite repository impls per entity group `[P]` (after SM-T6)
Each task implements the SQLite variant of that group's `core` port traits, mapping pg-isms
(`jsonb_set`→`json_set`, `->>`/`@>`→JSON1, drop `diesel::pg` DSL) per OC-4. Same trait signatures.

- **SM-T7 [P]** — account + account_tag (`AccountRegistration/Fetching/Updating/Deletion`, tag traits)
- **SM-T8 [P]** — tenant + tenant_tag (incl. `status Array<Jsonb>` → JSON TEXT)
- **SM-T9 [P]** — user (`UserRegistration/Fetching/Updating/Deletion`, `mfa` JSONB)
- **SM-T10 [P]** — token + session_token + `TokenInvalidation` (incl. magic-link `jsonb_set`→`json_set`)
- **SM-T11 [P]** — guest_role + guest_user (+ on-account) (incl. `permit_flags/deny_flags Array<Text>`)
- **SM-T12 [P]** — message: `LocalMessageWrite` / `LocalMessageReading` (feeds email_dispatcher)
- **SM-T13 [P]** — webhook (`propagations`, `headers` JSONB)
- **SM-T14 [P]** — error_code
- **SM-T15 [P]** — licensed_resource + `LicensedResourcesFetching` + `ProfileFetching`
- **SM-T16 [P]** — encryption_key (`EncryptionKeyFetching`) — envelope-encryption support
- **SM-T17** — `SqlAppModule` (sqlite) shaku registration wiring all the above (barrier: needs T7–T16)
- **Where (all):** `adapters/diesel/src/repositories/<entity>/*` (cfg-gated bodies), `repositories/mod.rs`.
- **Depends on:** SM-T6 (T7–T16 parallel); SM-T17 depends on T7–T16.
- **Reuses:** SM-T6 helpers; existing Postgres impls as behavioral reference.
- **Done when (each):** trait methods compile+pass under sqlite; CRUD round-trips on a temp DB.
- **Tests (each):** unit CRUD + the group's pg-ism-specific path (e.g. magic-link invalidation for T10).

---

## G3 — In-process cache (moka)

### SM-T18 — `adapters/cache` crate with redis + moka features
- **What:** New crate `myc-cache` housing the Redis impl (moved from/aliasing `kv_db`) and a moka impl,
  behind `redis` / `moka` features. `core` unchanged. (Alternative: `moka` feature on `kv_db`.)
- **Where:** `adapters/cache/` (new) or `adapters/kv_db/`.
- **Depends on:** SM-T1.
- **Done when:** crate builds under each feature; full mode still resolves the Redis impl identically.
- **Tests:** compile-only both features.

### SM-T19 — moka `KVArtifactRead`/`KVArtifactWrite` with per-key TTL (SM-R5)
- **What:** Implement both traits over `moka::future::Cache<String, String>` with an `Expiry` impl so
  the per-call `ttl` is honored per entry (not a global TTL).
- **Where:** `adapters/cache/src/moka_*.rs`.
- **Depends on:** SM-T18.
- **Reuses:** verified trait signatures (`get_encoded_artifact`, `set_encoded_artifact(key,value,ttl)`).
- **Done when:** set→get returns value; entry expires after its ttl; missing key → `NotFound`.
- **Tests:** unit — TTL expiry (advance/await), NotFound path.

---

## G4 — Local email transport

### SM-T20 — lettre `local-transport` feature + StubTransport impl (SM-R7)
- **What:** Enable lettre `stub-transport`/`file-transport` under a `local-transport` feature on
  `notifier`. Stub `RemoteMessageWrite` that renders the message and `tracing::info!`s subject,
  recipient, and any magic-link URL.
- **Where:** `adapters/notifier/Cargo.toml`, `adapters/notifier/src/repositories/*`.
- **Depends on:** SM-T1.
- **Reuses:** existing `RemoteMessageWrite` trait + `Message` DTO.
- **Done when:** stub impl compiles under `local-transport`; logs contain the magic-link URL.
- **Tests:** unit — stub send captures/logs body incl. URL.

### SM-T21 — FileTransport impl + transport selection (SM-R8)
- **What:** File `RemoteMessageWrite` (`.eml` to configured dir). Selection: SMTP if `[smtp]` present →
  else `file` if configured → else stub.
- **Where:** `adapters/notifier/src/repositories/*`, notifier config.
- **Depends on:** SM-T20.
- **Done when:** file mode writes a parseable `.eml`; SMTP still used when configured.
- **Tests:** unit — file written; selection precedence.

---

## G5 — Autogen secrets

### SM-T22 — Standalone secret source (keyring + encrypted-file fallback) (SM-R9, DEC-2)
- **What:** Resolve `token_secret` + JWT/HMAC secrets: explicit(env/config) → OS keyring → encrypted
  local file → generate-once-and-persist. Keyring failure (no backend) degrades gracefully to file
  (OC-2). Never regenerate if already persisted (protects KEK/HMAC — AD-004).
- **Where:** `lib/config` (new `standalone-secrets` feature) + wiring in `ports/api`.
- **Depends on:** SM-T1.
- **Reuses:** `SecretResolver` (explicit path), envelope `derive_key` patterns.
- **Done when:** first boot generates+persists; second boot reuses (same secret); keyring-absent falls
  back to file without panic.
- **Tests:** unit — generate/persist/reload identity; simulated keyring-absent fallback.

---

## G6 — Config + DI wiring

### SM-T23 — Standalone config shape (SM-R10)
- **What:** Under `standalone`, make `redis`/`smtp`/`queue`/`vault` optional/absent; interpret DB as a
  SQLite file path (`[sqlite] path` or reused `[diesel]`). Ship `settings/config.standalone.example.toml`.
- **Where:** `ports/api/src/models/config_handler.rs` (cfg-gated), `settings/`.
- **Depends on:** SM-T1.
- **Done when:** standalone parses a minimal TOML; full-mode `ConfigHandler` parsing is unchanged.
- **Tests:** unit — parse minimal standalone config; full-mode config test still passes.

### SM-T24 — cfg-gated `initialize_modules` for standalone (SM-R7 boot, wiring)
- **What:** Standalone `initialize_modules`: sqlite pool (auto-migrate) → SqlAppModule; moka → KVAppModule;
  stub/file notifier → NotifierModule; autogen secrets; spawn existing `email_dispatcher` (reads from
  sqlite). No hard-fail/panic on missing Redis/SMTP/Vault (those paths cfg-excluded).
- **Where:** `ports/api/src/main.rs`.
- **Depends on:** SM-T17, SM-T19, SM-T21, SM-T22, SM-T23.
- **Reuses:** existing `email_dispatcher`, shaku module types (OC-1 resolved — no new consumer).
- **Done when:** standalone binary boots against an empty working dir with no external services.
- **Tests:** integration — boot standalone, hit health endpoint.

---

## G7 — Packaging & E2E

### SM-T25 — `Dockerfile.standalone`
- **What:** Build `--no-default-features --features standalone`; minimal runtime base (distroless/cc or
  debian-slim) with only binary + templates + `ca-certificates`; NO `libpq-dev`; `/data` volume.
- **Where:** `Dockerfile.standalone` (+ optionally `docker-release.yml`).
- **Depends on:** SM-T24.
- **Done when:** image builds; `ldd` shows no libpq/libsqlite system deps.
- **Tests:** image build; `docker run` starts.

### SM-T26 — Zero-config E2E smoke (SM-R13)
- **What:** `docker run <standalone-image>` with no config → health OK, issue a JWT (magic-link, see URL
  in stdout), add a downstream route, proxy a request.
- **Where:** test script / docs.
- **Depends on:** SM-T25.
- **Done when:** the full onboarding path works under 2 min from a clean run.
- **Tests:** scripted smoke.

---

## G8 — Docs (can start early, finalize last)

### SM-T27 — Limitations + roadmap/marketing corrections
- **What:** Publish L-1…L-6 in the book before any "zero dependencies" claim; already corrected
  ROADMAP (C-4, C-7) — extend to marketing/docs; document the secrets file (L-6) and stub/file email (L-5).
- **Where:** `docs/book/src/*`, ROADMAP (done).
- **Depends on:** none (finalize after behavior lands).
- **Done when:** limitations documented; no marketing claim precedes them.

---

## Execution order summary

```
SM-T1 → SM-T2
SM-T1 → SM-T3 → SM-T4
                SM-T5 → SM-T6 → (SM-T7..T16 [P]) → SM-T17
SM-T1 → SM-T18 → SM-T19
SM-T1 → SM-T20 → SM-T21
SM-T1 → SM-T22
SM-T1 → SM-T23
(SM-T17, T19, T21, T22, T23) → SM-T24 → SM-T25 → SM-T26
SM-T27 anytime, finalize last
```

**Critical path:** G2 (SM-T3→T6→repo impls→T17). This is the bulk; land per-entity groups
incrementally behind the feature flag, keeping full mode green after every task.

## Traceability

| Task group | Requirements |
|---|---|
| G1 | SM-R11, SM-R14 |
| G2 | SM-R1, SM-R2, SM-R3, SM-R4, SM-R15 |
| G3 | SM-R5 |
| G4 | SM-R6, SM-R7, SM-R8 |
| G5 | SM-R9 |
| G6 | SM-R10, boot no-fail |
| G7 | SM-R12, SM-R13 |
| G8 | L-1…L-6, C-4, C-7 |
