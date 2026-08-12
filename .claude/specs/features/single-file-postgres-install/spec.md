# Single-file Postgres install for the 9.0.0 stable release

**Status:** Implemented — verified on Postgres 12/14/16, **awaiting user UAT before commit**
**Scope:** Medium (2 SQL files + docs, no Rust change, no architectural change)
**Blocks:** 9.0.0 stable release

## Files changed

| File | Change |
|---|---|
| `adapters/diesel_postgres/sql/up.sql` | Folded in the 3 missing migrations; two-phase structure with transaction; already-installed guard; guarded role/user creation; `CONCURRENTLY` removed |
| `adapters/diesel_postgres/sql/migrations/20260812_01_audit_tables_grants.sql` | **New** — grants the app role access to `instance_settings`/`resource_audit_log` on already-migrated databases (D-07) |
| `README.md`, `docs/book/src/02-installation.md`, `docs/book/src/18-cli.md`, `docker-compose.common.yaml` | Stale `postgres/sql/up.sql` path → `adapters/diesel_postgres/sql/up.sql`; installation doc now states there is nothing else to apply |
| `.claude/specs/codebase/CONVENTIONS.md` | New § "Postgres schema changes: two files, one commit" |
| `.claude/specs/features/postgres-only-mode/spec.md`, `.claude/specs/features/staff-bootstrap/design.md` | Marked the old "not folded" convention superseded |

No Rust file changed, so `cargo` gate checks are unaffected by this work.

---

## Problem

Postgres is the only backend whose schema is **not** self-installing. The SQLite adapter
compiles its migrations into the binary (`adapters/diesel_sqlite/src/migration.rs:10`,
`embed_migrations!("migrations")`) and `provision_database` runs them on every boot
(called from `ports/api/src/main.rs:112`). Postgres has no equivalent — `myc_diesel`'s
`migration` module exposes only `rotate_kek`/`migrate_dek`, which are KEK operations, not
schema ones. The operator must apply raw SQL by hand.

Today that means `psql -f up.sql` **plus** five files from `sql/migrations/` applied in
chronological order, three of which are not re-runnable. Downstream consumers have to
write their own wrapper to do it. A real one, `deploy/azure-vm/migrate.sh` in
`Biotrop/eva-natural-ai`, is ~100 lines that copies all six files out of this repo and
guards each non-idempotent step with a `to_regclass` table-existence check.

For a **stable** release that is the wrong first-run experience: there is no single
documented command that produces a working 9.0.0 database.

### What is actually missing from `up.sql`

Two of the five migrations were already folded in during the postgres-only-mode work, so
the gap is narrower than the migrations directory suggests:

| Migration | Folded into `up.sql`? |
|---|---|
| `20260421_01_envelope_encryption.sql` | **No** — `tenant` has no `encrypted_dek` / `kek_version` (`up.sql:74-82`) |
| `20260713_01_instance_settings.sql` | **No** — table absent |
| `20260713_02_resource_audit_log.sql` | **No** — table, 2 indexes, trigger fn + trigger all absent |
| `20260722_01_kv_artifact_cache.sql` | Yes — `up.sql:263-272` |
| `20260722_02_message_queue_claim_index.sql` | Yes — `up.sql:258-261` |

Both folded tables are referenced by `adapters/diesel_postgres/src/schema.rs` alongside
`instance_settings` (`:251`) and `resource_audit_log` (`:263`), so the Diesel schema
already assumes all five are present. A database built from `up.sql` alone today does not
match `schema.rs` — the app compiles against tables that do not exist.

### Secondary defects in `up.sql`, found while reading it

- **`CREATE ROLE` is not guarded and roles are cluster-global** (`up.sql:54-56`).
  Installing a second Mycelium database on the same Postgres cluster fails on
  `CREATE ROLE "service-role-mycelium"` — the role already exists from the first install.
- **No transaction.** A failure partway leaves a half-built schema that the
  "already installed" state cannot be distinguished from, and the operator has no clean
  retry.
- **`CREATE INDEX CONCURRENTLY`** at `up.sql:420,427` and `DROP INDEX CONCURRENTLY` at
  `:426`. On a fresh, empty database CONCURRENTLY buys nothing — there is no concurrent
  write traffic to avoid locking — and it is precisely what forbids wrapping the DDL in a
  transaction. The `DROP INDEX ... idx_account_meta_telegram_user_id_per_tenant` is
  cleanup for an index renamed in `c79c1f5d`; on a fresh database it never existed.
- **Documented path is stale.** `README.md:169` says `psql -f postgres/sql/up.sql`. That
  path does not exist — `postgres/` contains only `volume/` and `volume-2/`. The file is
  at `adapters/diesel_postgres/sql/up.sql`. `docker-compose.common.yaml:48` carries the
  same stale path in a commented-out mount.

---

## Requirements

| ID | Requirement |
|---|---|
| **R-01** | `up.sql` alone must produce a database schema **identical** to `up.sql` + the five `sql/migrations/*.sql` applied in chronological order. |
| **R-02** | Installation must be a single `psql` invocation with no wrapper script and no per-migration `-v` variables. |
| **R-03** | `sql/migrations/` must remain unchanged. Operators already running an `9.0.0-rc.x` database upgrade through those files; folding must not break that path. |
| **R-04** | Re-running `up.sql` against an already-installed database must be a safe, explicit no-op — not a partial-failure cascade. |
| **R-05** | `up.sql` must succeed on a cluster that already hosts another Mycelium database (role/user already present). |
| **R-06** | The schema DDL must be transactional: either the whole schema exists or none of it does. |
| **R-07** | The three tables folded in must receive the same grants as every other table. |
| **R-08** | The documented install command must point at the real file path. |

---

## Decisions

### D-01: Fold into `up.sql`; do not add a sibling `install.sql`

**User decision (2026-08-12).** `up.sql` becomes the complete 9.0.0 schema.

This reverses a convention recorded in `staff-bootstrap/design.md:145-153` and
`postgres-only-mode/spec.md:56` ("incremental files are not folded back into `up.sql`").
Two reasons it should be reversed rather than worked around:

1. The convention is **already broken in practice** — `kv_artifact` and
   `idx_message_queue_claim` were folded in (see table above), matching
   `postgres-only-mode/tasks.md:104` ("fold into `up.sql`"), which contradicts the other
   two documents. The repo has been inconsistent about this, not consistent.
2. `INTEGRATIONS.md:14` already advertises `up.sql` as "complete schema in one file". The
   fold makes the documentation true instead of aspirational.

The rejected alternative — a sibling `sql/install.sql` — duplicates ~459 lines of base
DDL across two files that diverge the first time someone adds a migration and updates
only one of them.

**Follow-up:** update `staff-bootstrap/design.md`, `postgres-only-mode/spec.md`, and
`CONVENTIONS.md` to record the new rule: *a new migration goes in `sql/migrations/` **and**
is folded into `up.sql` in the same commit.*

### D-02: Structure is two phases, not one transaction

`CREATE DATABASE` cannot run inside a transaction block, and `\c` reconnects. So:

- **Phase A (no transaction):** variable defaults, `CREATE DATABASE` via `\gexec`,
  `\c :"db_name"`, then the already-installed guard.
- **Phase B (`BEGIN` … `COMMIT`):** roles, extension, tables, constraints, views,
  indexes, trigger, grants.

### D-03: R-04 is met by an early-exit guard, not by making 40 constraints idempotent

Postgres has no `ALTER TABLE ... ADD CONSTRAINT IF NOT EXISTS`. Making the ~40
`ADD CONSTRAINT` statements re-runnable would mean ~40 `DO $$ ... $$` blocks — a large,
risky rewrite of a file whose job is first-install.

Instead, phase A probes `to_regclass('public.account')`; if the schema is present, `up.sql`
prints a message pointing at `sql/migrations/` for upgrades and `\quit`s before touching
anything. Re-running becomes a clean no-op with a useful message.

`CREATE ROLE` / `CREATE USER` **are** individually guarded (two `DO` blocks), because R-05
is a different case: the role exists but this database's schema does not, so the early-exit
guard does not fire.

`CREATE USER` when the user already exists leaves the existing password alone rather than
resetting it — silently rotating a credential an operator did not ask to rotate is worse
than a no-op.

### D-04: `CREATE INDEX CONCURRENTLY` becomes plain `CREATE INDEX`

Required by R-06 (D-02 phase B). Safe because phase B only ever runs on a database with no
`account` rows — the guard in phase A guarantees it. The legacy
`DROP INDEX CONCURRENTLY IF EXISTS idx_account_meta_telegram_user_id_per_tenant` is removed
from `up.sql`: it is meaningless on a fresh database. It stays reachable for upgraders
because `sql/migrations/` is untouched (R-03).

### D-07: the migrated path had a grant bug; it gets its own migration

**Found by the R-01 diff, not by reading.** With the fold done, the two schemas were
byte-identical except for two lines present only in the consolidated database:

```
GRANT ALL ON TABLE public.instance_settings  TO "service-role-mycelium";
GRANT ALL ON TABLE public.resource_audit_log TO "service-role-mycelium";
```

The divergence is the **migrated** path being wrong, not the consolidated one. `up.sql`'s
`GRANT ALL ON ALL TABLES IN SCHEMA public` is evaluated at execution time and sits at the end
of the file, so it only ever covered tables that existed when it ran. `instance_settings` and
`resource_audit_log` are created afterwards by their own migrations, and neither carries a
`GRANT`. `kv_artifact` is unaffected precisely because `20260722_01` *does* grant explicitly —
which is why it matched.

So on **every** database built from `up.sql` + migrations, the app role can read every table
except those two. Deployments where the app connects as a superuser never notice — that
includes `eva-natural-ai`, whose `migrate.sh` comments record that `db_user`/`db_role` are
created but unused. A deployment following the README, where the app connects as `db_user`
(a member of `db_role`), fails on the staff-bootstrap claim and on every audit-log write.

Fixed by a **new** migration, `20260812_01_audit_tables_grants.sql`, rather than by editing
the two shipped files. Rewriting an already-applied migration would leave existing databases
silently wrong — nothing here tracks which migrations ran, so an operator who applied
`20260713_01` months ago would never re-read it. A new dated file is the mechanism that
actually reaches them, and it is how this repo evolves schema anyway. R-03 is preserved
literally: no existing migration was touched.

### D-05: SQLite is out of scope

`adapters/diesel_sqlite` self-installs via `embed_migrations!` + `provision_database` on
every boot, and its `2026-07-06-000000_init` already carries `encrypted_dek`/`kek_version`
inline. Standalone-mode users run no SQL by hand. Nothing to do.

### D-06: Embedded Postgres migrations are deferred, not adopted

Giving the Postgres adapter `embed_migrations!` would eliminate this whole class of problem,
but it changes production startup semantics — every replica in a multi-pod deployment would
race to run migrations at boot (see `project_production_topology`: Kubernetes, multi-pod).
That is a larger design question than the release needs. Deferred to ROADMAP.

---

## Verification — results

"It ran without errors" does not prove R-01. Both schemas were built on a throwaway
container and diffed:

```
DB A  ← up.sql@HEAD + the 6 sql/migrations/*.sql in chronological order (eva's exact vars)
DB B  ← consolidated up.sql alone
pg_dump --schema-only on each → strip comments/blank lines/pg_dump \restrict tokens → diff
```

**Run on Postgres 12, 14 and 16** — 12 because `docker-compose.common.yaml:39` pins
`postgres:12`, 14 because that is the documented minimum, 16 as the current mainstream.

| Requirement | Result |
|---|---|
| **R-01** schema equivalence | **PASS** on all three versions — 420 normalised lines, identical. Zero table/column/index/constraint/view/trigger differences. Before adding `20260812_01`, the only delta was the two missing grants (D-07). |
| **R-02** single invocation | PASS — one `psql -f up.sql -v db_password=…`, no wrapper, no per-migration vars. |
| **R-04** re-run is a no-op | PASS — guard fires, prints the "already installed → use sql/migrations/" notice, `\quit`s before any DDL. |
| **R-05** role already exists | PASS — DB B installed on a cluster where DB A had already created `service-role-mycelium`/`mycelium-user`; both guards reported reuse and the install completed. |
| **R-06** atomicity | PASS — injected a bogus column type into `account_tag`; after the failure `information_schema.tables` in the target database returned **0** rows. |
| **R-07** grants on folded tables | PASS — `instance_settings`, `resource_audit_log`, `kv_artifact` each hold the full grant set for `db_role` (checked via `information_schema.role_table_grants`, since `pg_dump --no-acl` hides grants). |
| Immutability trigger survives the fold | PASS — insert succeeds, `UPDATE` raises `resource_audit_log is immutable: UPDATE not allowed`. |
| Grant coverage is complete on a fresh install | PASS — queried every `pg_class` relation of kind table/view/matview/sequence in `public` for a missing `service-role-mycelium` grant: **empty**. Both views included. |

### `20260812_01` must be applied by the table owner

Checked separately, because the R-01 run only ever applied migrations as the superuser.
Applying `20260812_01` as `mycelium-user` against an rc.x database fails:

```
ERROR:  permission denied for table instance_settings
```

`GRANT` requires ownership or `WITH GRANT OPTION`, and in an rc.x database all 21 tables are
owned by the `postgres` superuser that ran `up.sql` — not by the app's `db_user`. **This is
pre-existing convention, not something the fix introduces:** the already-shipped
`20260722_01` fails identically (`ERROR: must be owner of table kv_artifact`), as does
`up.sql` itself. Documented in the migration header and in
`docs/book/src/02-installation.md` rather than worked around; there is no way to grant
without the privilege to grant.

Script: `verify.sh` (session scratchpad — not committed; see the deferred CI item below).

**Deferred:** a CI job asserting the two paths produce identical dumps is the only durable
guarantee that the fold stays correct as migrations are added. The convention in
`CONVENTIONS.md` states the rule; only CI enforces it. Worth doing; out of scope here.

---

## Out of scope

- The release itself. `release-stable.yml` requires `github.ref == 'refs/heads/main'` and
  the gateway is on `develop`, so a `develop` → `main` PR is a prerequisite. Sequencing is
  a separate decision (rc.13 first, or straight to stable — **open question**, latest tag
  is `9.0.0-rc.12`, rc.8 and rc.13 do not exist).
- Migration-tracking table / `schema_migrations` bookkeeping (see D-06).
- SQLite (D-05).
