# Tasks: Staff Bootstrap (Autonomous First-Login Onboarding)

**Feature:** staff-bootstrap · **Spec:** `./spec.md` · **Design:** `./design.md`
**Branch:** `feat/staff-bootstrap` (gateway submodule, created from `develop`)
**Status:** Implemented (all tasks below done); gate checks green. Not yet committed
  (`commit-validation.md` — awaiting manual operator test/approval).
**Created:** 2026-07-13

**Revision note (2026-07-13, post-implementation):** SB-T1/T2/T3/T4/T7/T9/T10/T15 originally
targeted a fixed-column singleton row (`staff_bootstrap_status`/`claimed_by_user_id`/`claimed_at`).
Per user feedback, `instance_settings` was reworked into a generalized key/value settings table
(see spec.md DEC-3/DEC-6, design.md §2/§3) before this feature shipped. Task descriptions below are
updated in place to reflect what was actually built — the generic `InstanceSetting` DTO, the
two-trait `Fetching`/`Registration` port pair (no `Updating` trait), and claim-by-insert semantics.

**Revision note 2 (2026-07-13, same day, user request — SB-R13):** added `created_by`/`updated_by`
(JSON, holding `WrittenBy`) to `instance_settings`, and generalized `WrittenBy` itself (`core/src/
domain/dtos/written_by.rs`) so `id`/`from` are optional and an independent, base64-encoded `email`
field carries identity when no User/Account exists yet. This made `StaffBootstrapClaim` (SB-T1's
payload struct) redundant and it was removed — `created_by`/`created` on the row already capture
who claimed it and when. `SB-T2`'s `get_or_create` gained a third `created_by: Option<WrittenBy>`
parameter; `SB-T3`/`SB-T4`'s migrations gained the two columns; `SB-T10`/`SB-T15` now build a
`WrittenBy` from the email already in scope at their call sites instead of a bespoke payload.

Invariant on every task: `request_magic_link`/`verify_magic_link` and their tests stay byte-identical
(SB-R10). Gate for all Rust tasks: `cargo fmt --all -- --check` + `cargo build --workspace` +
`cargo test --workspace --all`, and (from G1 onward) also `cargo build --no-default-features
--features standalone`.

`[P]` = parallelizable with siblings once dependencies are met.

Legend for each task: **What / Where / Depends on / Reuses / Done when / Tests**.

---

## G1 — Domain model, ports, and dual-adapter schema

### SB-T1 — `InstanceSetting` DTO + `StaffBootstrapClaim` payload (revised)
- **What:** Add the generic domain DTO (`InstanceSetting { key, value, created, updated }`), the
  `STAFF_BOOTSTRAP_KEY` const, and the `StaffBootstrapClaim` payload struct that this feature
  serializes into/out of the generic `value` (design §2). No `StaffBootstrapStatus` enum — row
  presence under `STAFF_BOOTSTRAP_KEY` is the signal.
- **Where:** `core/src/domain/dtos/instance_settings.rs` (+ `mod.rs` re-export).
- **Depends on:** none.
- **Reuses:** existing DTO style (`MagicLinkTokenMeta` as reference).
- **Done when:** compiles, `Serialize`/`Deserialize` round-trip tests pass for both types.
- **Tests:** unit — serde round-trip for `InstanceSetting` and `StaffBootstrapClaim`.

### SB-T2 — Port traits: fetching / registration (revised — two, not three)
- **What:** `InstanceSettingsFetching::get(key)`, `InstanceSettingsRegistration::get_or_create(key,
  value)` (design §2) — one trait per verb. No `InstanceSettingsUpdating`: claiming a key *is*
  creating it, so there's no separate CAS-update step to model.
- **Where:** `core/src/domain/entities/instance_settings/{instance_settings_fetching,
  instance_settings_registration}.rs`.
- **Depends on:** SB-T1.
- **Reuses:** `TokenRegistration`/`TokenInvalidation` split as the structural precedent.
- **Done when:** traits compile with `Interface + Send + Sync`, no adapter imports (hexagonal check).
- **Tests:** none yet (traits only — `mockall` mocks come with the use-cases that consume them).

### SB-T3 [P] — Postgres: migration + model + repo impls (revised — key/value)
- **What:** Additive migration (design §3) creating `instance_settings (key PK, value JSONB,
  created, updated)`, Diesel model/schema entry, and the two repo trait impls, including the
  `INSERT ... ON CONFLICT (key) DO NOTHING` claim SQL.
- **Where:** `adapters/diesel_postgres/sql/migrations/20260713_01_instance_settings.sql`,
  `adapters/diesel_postgres/src/{models,repositories}/instance_settings/`.
- **Depends on:** SB-T2.
- **Reuses:** existing repo-impl conventions (`token_registration.rs`/`token_invalidation.rs` as
  structural reference).
- **Done when:** code compiles, matches the query-builder style used elsewhere (`diesel::insert_into`
  + `.on_conflict_do_nothing()` — not raw `sql_query`, since `instance_settings` has plain typed
  columns, key/value, not JSONB-path predicates).
- **Tests:** none at the repo level — **verified there is no existing precedent for real-Postgres
  repo tests anywhere in `diesel_postgres`** (zero `#[cfg(test)]` modules in `repositories/`); the
  codebase's actual convention is DB-level correctness tested on the SQLite side (temp file +
  `embed_migrations`, per `diesel_sqlite`'s ~26 existing tests) and use-case logic tested with
  mockall (SB-T7/T9/T10). Do not build a bespoke Postgres test harness for this feature alone; the
  concurrency property (SB-R2) is exercised on the SQLite side instead (SB-T4) and via manual
  operator verification against a real full-mode deploy (handed to the user per commit-validation).

### SB-T4 [P] — SQLite: migration + model + repo impls (revised — key/value)
- **What:** Same as SB-T3, targeting `diesel_sqlite`'s TEXT-based type mapping (design §3). Unlike
  Postgres, `created` has no DB-side default — supplied explicitly on insert via
  `naive_timestamp_to_text(&Local::now().naive_utc())` (a `datetime('now')` DB default produces
  text this codebase's parser rejects — caught by this task's own round-trip test).
- **Where:** `adapters/diesel_sqlite/migrations/2026-07-13-000000_instance_settings/{up,down}.sql`,
  `adapters/diesel_sqlite/src/{models,repositories}/instance_settings/`.
- **Depends on:** SB-T2.
- **Reuses:** existing `diesel_sqlite` type-mapping helpers (`types.rs`) for `Jsonb↔TEXT`, and its
  temp-file + `embed_migrations!` test harness pattern (mirrors `SM-T5`).
- **Done when:** embedded migration creates the table on an empty temp DB; `get_or_create` on a new
  key returns `Created`; a second `get_or_create` on the same key (different value) returns
  `NotCreated` and does not overwrite the winner's value; fetching a different, never-claimed key
  returns `NotFound`. SQLite's global write lock serializes concurrent callers by itself, so a true
  race test isn't meaningful here — a sequential idempotency test is sufficient and is this
  feature's actual DB-level correctness test (see SB-T3's note).
- **Tests:** unit/integration against a temp SQLite file, following `diesel_sqlite`'s existing test
  style exactly.

### SB-T5 — Wire into shaku DI (both adapters)
- **What:** Add the two new `InstanceSettings*SqlDbRepository` structs to the `components = [...]`
  list in each adapter's own `module! { pub SqlAppModule { ... } }` macro. **No `ports/api` change
  needed** — `ports/api/src/models/active_backend_modules.rs` only re-exports whichever
  `SqlAppModule` type is active via `#[cfg(feature = ...)]`; it never lists components (corrects an
  earlier draft of this task).
- **Where:** `adapters/diesel_postgres/src/repositories/mod.rs`,
  `adapters/diesel_sqlite/src/repositories/mod.rs`.
- **Depends on:** SB-T3, SB-T4.
- **Done when:** both `cargo build` (default) and `cargo build --no-default-features --features
  standalone` compile with the new components registered in their respective `module!` blocks.
- **Tests:** compile-only (shaku validates the DI graph at compile time via the `module!` macro).

---

## G2 — Config field + boot-time row bootstrap

### SB-T6 — `staff_bootstrap_secret` config field
- **What:** Add `staff_bootstrap_secret: Option<SecretResolver<String>>` to `AccountLifeCycle`
  (design §4). Optional — absent by default.
- **Where:** `core/src/models/account_life_cycle_config.rs`.
- **Depends on:** none.
- **Reuses:** existing `SecretResolver` config pattern (same as `token_secret`, SMTP credentials).
- **Done when:** config parses with and without the field present; defaults to `None`.
- **Tests:** unit — TOML round-trip, field-absent case (mirrors `account_life_cycle_config.rs`'s
  existing `#[cfg(test)]` style).

### SB-T7 — `staff_bootstrap_is_pending` use-case (revised — fetch-only, renamed)
- **What:** Thin, fetch-only use-case calling `InstanceSettingsFetching::get(STAFF_BOOTSTRAP_KEY)`
  (design §5) and mapping `NotFound → true` (pending), `Found → false` (claimed). No crypto, no
  per-replica branching, **no insert at all** — the generalized table has nothing to pre-create.
- **Where:** `core/src/use_cases/super_users/staff/bootstrap/staff_bootstrap_is_pending.rs`.
- **Depends on:** SB-T2 (traits), SB-T1.
- **Done when:** calling it against an empty DB returns `true`; calling it once the key exists
  returns `false`.
- **Tests:** unit with a mocked `InstanceSettingsFetching` (mockall).

### SB-T8 — Boot sequence integration + logging
- **What:** Call SB-T7 from `ports/api/src/main.rs` after DI wiring, before `HttpServer::run()`;
  if `staff_bootstrap_secret` is configured and SB-T7 reports pending, log the claim URL (built from
  `life_cycle_settings.domain_url`, design §1 OC-3, §8) — never the secret itself. If the secret is
  unset, or the key already exists, log nothing bootstrap-related (SB-R4). **Non-fatal (SB-R12):** if
  `staff_bootstrap_is_pending` errors (e.g. table doesn't exist because the operator hasn't
  applied the migration yet — design §3), log the error and let boot continue; do not propagate `?`
  into a startup-aborting error.
- **Where:** `ports/api/src/main.rs`.
- **Depends on:** SB-T7, SB-T6, SB-T5.
- **Done when:** four boot scenarios behave per spec SB-R4/SB-R12: (a) secret configured + pending →
  logs claim URL; (b) secret unset → silent, regardless of key state; (c) secret configured +
  already claimed → silent; (d) `instance_settings` table missing/DB error → logs an error, gateway
  still boots and serves other requests normally.
- **Tests:** integration — the four scenarios above, asserting on captured `tracing` output and,
  for (d), that the HTTP server still comes up.

---

## G3 — Bootstrap claim use-cases

### SB-T9 — `validate_bootstrap_secret` use-case
- **What:** Constant-time-compare the presented secret against the resolved
  `staff_bootstrap_secret` using `subtle::ConstantTimeEq` (404-mapped error if unset — SB-R4), then
  fetch `STAFF_BOOTSTRAP_KEY`, require `NotFound` (design §5, §1 OC-4).
- **Where:** `core/src/use_cases/super_users/staff/bootstrap/validate_bootstrap_secret.rs`. Add
  `subtle.workspace = true` to `core/Cargo.toml` (already a workspace dependency, already used
  directly by `lib/http_tools` for the same purpose).
- **Depends on:** SB-T1, SB-T2, SB-T6.
- **Reuses:** `lib/http_tools/src/telegram/verify_webhook_secret.rs`'s exact comparison pattern.
- **Done when:** valid + key absent → `Ok(())`; wrong secret → expected error; secret unset →
  expected (404-mapped) error regardless of key state; key already exists (even with the correct
  secret, e.g. a replayed request) → expected error (SB-R8).
- **Tests:** unit, mocked repo — table-driven over the four cases above.

### SB-T10 — `claim_staff_bootstrap` use-case (insert-first, no transaction)
- **What:** **D-1 revised — no transaction** (there is no cross-repo transaction anywhere in this
  codebase; verified against `create_seed_staff_account`'s two-independent-calls precedent). Serialize
  a `StaffBootstrapClaim` and call `InstanceSettingsRegistration::get_or_create(STAFF_BOOTSTRAP_KEY,
  ...)` **first**; only on `Created` (this call won the key) proceed to upgrade/create the caller's
  `Account` as `AccountType::Staff`; on `NotCreated`, return "already initialized" immediately
  without calling the Account repo at all.
- **Where:** `core/src/use_cases/super_users/staff/bootstrap/claim_staff_bootstrap.rs`. May require
  factoring the account-creation half of `create_seed_staff_account` (lines ~86-102) into a small
  reusable helper — confirm during implementation whether extraction is worth it vs. a second,
  near-identical call (object-calisthenics: don't extract prematurely if the two call sites end up
  needing genuinely different account-creation semantics, e.g. one has a password, one doesn't).
- **Depends on:** SB-T9, SB-T2, existing `AccountRegistration` port (no change).
- **Done when:** happy path (`Created`) creates a Staff account; a mocked `NotCreated` response
  from `InstanceSettingsRegistration` results in the Account repo mock **never being called** and an
  "already initialized" error returned.
- **Tests:** unit with mockall — table-driven over `Created`/`NotCreated`, asserting call counts on
  the Account repo mock. This is a trivial, fast test now that there's no transaction to exercise.

---

## G4 — REST endpoints and templates

### SB-T11 — `GET /_adm/instance/bootstrap` (claim page)
- **What:** Handler: 404 if `staff_bootstrap_secret` unset or the `staff_bootstrap` key already
  exists; else renders `web/instance-bootstrap-claim` (design §6). Note this GET has no secret/token
  in the query string
  to check yet — the secret is submitted by the operator on the subsequent POSTs (SB-T12/13), so
  this handler only needs the "is bootstrap enabled at all" check, not full `validate_bootstrap_secret`.
- **Where:** `ports/api/src/rest/instance/bootstrap_claim_page.rs` + `mod.rs`.
- **Depends on:** SB-T7, SB-T14 (templates — can stub with a placeholder template until SB-T14
  lands if run in parallel).
- **Reuses:** `magic-link-display.html`'s Tera-render call pattern.
- **Done when:** enabled+pending → 200 HTML with secret+email form; disabled/claimed → 404, not a 500.
- **Tests:** integration (actix test server) — both branches.

### SB-T12 — `POST /_adm/instance/bootstrap/request-code`
- **What:** Handler: `validate_bootstrap_secret` → call **unmodified** `request_magic_link` →
  generic `{sent:true}` response (design §6, SB-R6).
- **Where:** `ports/api/src/rest/instance/bootstrap_request_code.rs`.
- **Depends on:** SB-T9.
- **Reuses:** `request_magic_link` as-is — zero changes to that use-case (SB-R10 check point).
- **Done when:** valid secret + any email → `200 {sent:true}`; invalid secret/disabled/claimed →
  error, no email sent (verify via test double on the notifier port).
- **Tests:** integration — assert `request_magic_link` invocation only on valid secret + pending.

### SB-T13 — `POST /_adm/instance/bootstrap/complete`
- **What:** Handler: `validate_bootstrap_secret` → **unmodified** `verify_magic_link` → on success,
  `claim_staff_bootstrap` → return the same `MyceliumLoginResponse` shape as a normal magic-link
  verify (design §6, SB-R7).
- **Where:** `ports/api/src/rest/instance/bootstrap_complete.rs`.
- **Depends on:** SB-T10, SB-T12 (same validation call, same code path expectations).
- **Done when:** full happy path (request-code → display page → complete) yields a JWT and a Staff
  account; a concurrent second "complete" call (simulating two operators racing) yields "already
  initialized" for the loser, and the loser's underlying `User` row (if newly created by
  `verify_magic_link`) is left as an ordinary non-Staff user (accepted per OC-1) — not deleted, not
  erroring the whole request differently than any other already-initialized attempt.
- **Tests:** integration — full flow end-to-end, plus the race scenario.

### SB-T14 [P] — Templates
- **What:** `instance-bootstrap-claim.html`, `instance-bootstrap-error.html` (design §7).
- **Where:** `templates/web/`.
- **Depends on:** none (pure static asset, can start immediately/parallel with G1-G3).
- **Reuses:** `magic-link-display.html`'s CSS/card layout.
- **Done when:** both render correctly via Tera with representative context values.
- **Tests:** none (visual/manual check sufficient, same as existing templates).

---

## G5 — CLI consistency (SB-R9)

### SB-T15 — Hook the bootstrap claim into `create-seed-account`
- **What:** After a successful seed-account creation, best-effort call
  `InstanceSettingsRegistration::get_or_create(STAFF_BOOTSTRAP_KEY, claim_value)` (same call the web
  flow's claim uses); log (don't fail the command) on error.
- **Where:** `ports/cli/src/cmds/accounts.rs` (call site only — `create_seed_staff_account` itself
  stays untouched per DEC-1).
- **Depends on:** SB-T2, SB-T3, SB-T4, SB-T5.
- **Done when:** running `create-seed-account` against a not-yet-claimed key creates it; a failure
  to write `instance_settings` (simulate DB error) still leaves the CLI command succeeding, with a
  logged warning.
- **Tests:** integration — CLI happy path + simulated instance_settings-write failure.

---

## G6 — Non-regression and verification

### SB-T16 — Non-regression + full gate
- **What:** Diff `request_magic_link.rs`/`verify_magic_link.rs` (and their `#[cfg(test)]` modules)
  against `develop` — must be empty. Run the full and standalone gate checks (top banner). Also
  verify the "secret unset → fully silent, zero new behavior" regression case (design §11).
- **Where:** n/a (verification task).
- **Depends on:** all of G1-G5.
- **Done when:** diff is empty; `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test
  --workspace --all` all green; `cargo build --no-default-features --features standalone` green.
- **Tests:** the gate commands themselves, plus SB-T3/SB-T10's concurrency tests re-run under
  `--test-threads=1` and default parallelism both, to catch any test-order flakiness in the CAS logic.

### SB-T17 — Docs
- **What:** Add a short section to the ROADMAP.md M3 entry (mirroring the existing Magic
  Link/Standalone Mode bullet style) and, if the project's doc-book convention applies here
  (`docs/book/src/...`, as standalone mode did), a page describing the bootstrap flow for
  operators, including how to set `staff_bootstrap_secret`.
- **Where:** `.claude/specs/project/ROADMAP.md`, `docs/book/src/` (path TBD at implementation time).
- **Depends on:** SB-T16.
- **Done when:** an operator unfamiliar with the codebase can follow the doc end-to-end against a
  fresh deploy.
- **Tests:** none (documentation).
