# Resource Audit Log Tasks

**Design**: `.claude/specs/features/audit-log/design.md`
**Status**: Implemented — all 52 tasks done, full workspace gate green (fmt/build/test, both
`postgres-backend` and `standalone` feature sets), 2026-07-13. Not committed yet — awaiting manual
user test/approval per `commit-validation.md`. See `.claude/specs/project/STATE.md` AD-006 for the
execution summary, including a correctness fix (premature audit emission) and a multi-agent
`git stash` incident that was found and resolved mid-execution.

Scope confirmed with the user: every write use case except `error_code` (Auto-Sizing: Complex —
full breakdown, parallel plan, per-task verification). Package names used in gate commands:
`myc-core` (core), `mycelium-diesel` (Postgres adapter), `mycelium-diesel-sqlite` (SQLite adapter),
`mycelium-api` (ports/api, default feature `postgres-backend`, alt feature `standalone`).

---

## Execution Plan

### Phase 1: Foundation (mostly parallel)

```
T1 [P] ─┐
T2 [P] ─┼──→ T4 ──→ T5 ──→ (feeds instrumentation phases)
T3 [P] ─┘      └───→ T6 ──→ (feeds T11)
```

### Phase 2: Adapters, dispatcher, wiring, endpoint (sequential — shared files)

```
T4 ──→ T7 [P] ─┐
T4 ──→ T8 [P] ─┼──→ T9 ──→ T10 ──→ T11
                └──────────┘
```

### Phase 3: Instrumentation (parallel across domain lanes, sequential within each lane)

```
T10, T5 ──┬──→ Account lane:    T12 → T13 → ... → T28  [P vs other lanes]
          ├──→ Tenant lane:     T29 → T30 → ... → T36  [P vs other lanes]
          ├──→ Guest-role lane: T37 → T38 → ... → T42  [P vs other lanes]
          ├──→ User lane:       T43 → T44             [P vs other lanes]
          ├──→ Webhook lane:    T45 → T46 → T47        [P vs other lanes]
          ├──→ Account-meta lane: T48 → T49 → T50      [P vs other lanes]
          └──→ Tenant-meta lane:  T51 → T52            [P vs other lanes]
```

Each lane is one continuous sub-agent run, not N independent sub-agents, because use cases in the
same domain are very likely to share a REST/RPC handler file (e.g., all `users_manager/account`
endpoints in one router module) — parallelizing within a lane risks two agents editing the same
handler file at once. Lanes themselves touch disjoint files/domains, so they run concurrently.

---

## Task Breakdown

### T1: Postgres migration — `resource_audit_log` table, indexes, trigger [P]

**What**: Add a new SQL migration creating `resource_audit_log` exactly as specified in
`design.md`'s Data Models section (table, the two indexes, the `prevent_resource_audit_log_mutation`
function, the `trg_resource_audit_log_immutable` trigger), plus regenerate `schema.rs` via Diesel
CLI.
**Where**: `adapters/diesel_postgres/sql/migrations/<timestamp>_resource_audit_log.sql`,
`adapters/diesel_postgres/src/schema.rs` (regenerated).
**Depends on**: None
**Reuses**: `telegram_identity_audit`'s table/index shape (`adapters/diesel_postgres/sql/up.sql:417-431`) as the closest structural precedent.
**Requirement**: AUDIT-01

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] Migration file matches the schema in `design.md` verbatim (column names/types/constraints).
- [ ] `schema.rs` contains the generated `resource_audit_log` table macro.
- [ ] Applying the migration against a local dev Postgres succeeds with no errors.
- [ ] A manual `UPDATE resource_audit_log SET metadata = '{}' WHERE id = <any>` (after inserting one row manually) is rejected by the trigger.
- [ ] Gate check passes: `cargo build -p mycelium-diesel`.

**Tests**: none (SQL migration — verified by manual application, not a Rust unit test).
**Gate**: build

**Verify**:
```bash
docker-compose up -d postgres   # from modules/mycelium-api-gateway/
diesel migration run
psql "$DATABASE_URL" -c "INSERT INTO resource_audit_log (resource_type, resource_id, event, performed_by, created_at) VALUES ('account', gen_random_uuid(), 'created', '{}'::jsonb, now());"
psql "$DATABASE_URL" -c "UPDATE resource_audit_log SET metadata = '{\"x\":1}' WHERE resource_type = 'account';"
# expect: ERROR: resource_audit_log is immutable: UPDATE not allowed
```

**Commit**: `feat(diesel): add immutable resource_audit_log table and trigger`

---

### T2: SQLite migration — `resource_audit_log` table, indexes, trigger [P]

**What**: Same table/index/trigger semantics as T1, translated to SQLite syntax. Before writing,
open an existing `adapters/diesel_sqlite` migration to confirm the actual SQLite equivalents for
`UUID`/`JSONB`/`TIMESTAMPTZ`/`gen_random_uuid()` used elsewhere in that adapter — do not assume
Postgres syntax ports 1:1 (per design.md's "Open Items to Verify").
**Where**: `adapters/diesel_sqlite/sql/migrations/<timestamp>_resource_audit_log.sql` (or
wherever that adapter's existing migrations live — confirm path), `adapters/diesel_sqlite/src/schema.rs` (regenerated).
**Depends on**: None
**Reuses**: Whatever type/trigger conventions the existing `diesel_sqlite` migrations already established for other tables.
**Requirement**: AUDIT-02

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] Migration mirrors T1's columns/constraints using this backend's real type conventions (verified against an existing migration, not assumed).
- [ ] Two triggers (`BEFORE UPDATE`, `BEFORE DELETE`, each `RAISE(ABORT, ...)`) reject mutation.
- [ ] `schema.rs` regenerated for SQLite contains the table.
- [ ] Gate check passes: `cargo build -p mycelium-diesel-sqlite`.

**Tests**: none.
**Gate**: build

**Verify**: Apply the migration against a local SQLite file, insert one row, attempt `UPDATE`/`DELETE`, confirm both are rejected.

**Commit**: `feat(diesel-sqlite): add immutable resource_audit_log table and trigger`

---

### T3: Core DTOs — resource type, event kind, log row, new-event [P]

**What**: Create the four DTO files exactly as specified in `design.md`'s Components section
(`ResourceAuditResourceType`, `ResourceAuditEventKind`, `ResourceAuditLog`, `NewResourceAuditLogEvent`), each `Serialize`/`Deserialize`/`Clone`/`Debug`, following `written_by.rs` as the closest existing shape to mirror (derive list, `utoipa::ToSchema` if that's the existing convention for DTOs exposed over the API — confirm against `written_by.rs` or `account_type.rs`).
**Where**: `core/src/domain/dtos/resource_audit_log/{resource_audit_resource_type,resource_audit_event_kind,resource_audit_log,new_resource_audit_log_event}.rs`, plus `mod.rs` that only declares/re-exports the four submodules (screaming-architecture rule 5 — no catch-all logic in `mod.rs`).
**Depends on**: None
**Reuses**: `core/src/domain/dtos/written_by.rs` (shape + derive pattern), `core/src/domain/dtos/account_type.rs` (enum-with-payload pattern, for reference on how a tagged enum is declared here).
**Requirement**: AUDIT-03

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] All four types compile and are re-exported from `core::domain::dtos`.
- [ ] `ResourceAuditResourceType` has exactly the 7 variants from `design.md`; `ResourceAuditEventKind` has exactly 3.
- [ ] Unit tests cover serde round-trip for each enum variant and for `ResourceAuditLog`/`NewResourceAuditLogEvent`.
- [ ] Gate check passes: `cargo test -p myc-core resource_audit_log`.
- [ ] Test count: at least 4 tests pass (one round-trip per new type).

**Tests**: unit
**Gate**: quick (`cargo test -p myc-core resource_audit_log`)

**Commit**: `feat(core): add resource audit log DTOs`

---

### T4: Core ports — `ResourceAuditLogRegistration`, `ResourceAuditLogFetching`

**What**: Define the two trait ports exactly as specified in `design.md` (only these two — no
`Updating`/`Deletion` trait exists for this resource, by design).
**Where**: `core/src/domain/entities/resource_audit_log/{resource_audit_log_registration,resource_audit_log_fetching}.rs` (+ `mod.rs` re-exports only).
**Depends on**: T3
**Reuses**: `core/src/domain/entities/profile/profile_fetching.rs` as the minimal `#[async_trait] pub trait X: Interface + Send + Sync` shape to copy; `FetchManyResponseKind` from `mycelium-base`.
**Requirement**: AUDIT-03

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] Both traits compile, `Interface + Send + Sync`, `#[async_trait]`.
- [ ] `create` returns `Result<(), MappedErrors>`; `list_by_resource`/`list_by_tenant` return `Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors>`.
- [ ] Gate check passes: `cargo build -p myc-core`.

**Tests**: none (trait declarations only — nothing to unit test until an implementation exists).
**Gate**: build

**Commit**: `feat(core): add resource audit log port traits`

---

### T5: `emit_resource_audit_event` shared helper

**What**: Implement the helper exactly as specified in `design.md` — takes the port, builds
`NewResourceAuditLogEvent` with `Utc::now()` captured synchronously, calls `create`, and swallows
any `Err` into a `tracing::warn!` instead of propagating it. Returns `()`, not a `Result`.
**Where**: `core/src/use_cases/shared/audit/emit_resource_audit_event.rs` (+ `mod.rs`).
**Depends on**: T4
**Reuses**: Nothing new; this IS the reusable unit every instrumentation task calls.
**Requirement**: AUDIT-04

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] Function signature matches `design.md` exactly.
- [ ] Unit test with a `mockall`-mocked `ResourceAuditLogRegistration` asserts `create` is called once with the expected `NewResourceAuditLogEvent` fields.
- [ ] Unit test asserts that when the mock returns an `Err`, the helper does not panic and does not propagate the error (function still returns `()`).
- [ ] Gate check passes: `cargo test -p myc-core emit_resource_audit_event`.
- [ ] Test count: at least 2 tests pass.

**Tests**: unit
**Gate**: quick

**Commit**: `feat(core): add emit_resource_audit_event helper`

---

### T6: `fetch_resource_audit_trail` read use case + permission rule

**What**: Implement the permission-branching read use case exactly as specified in `design.md`.
Before writing the tenant-manager branch, open `core/src/domain/dtos/profile/mod.rs` and confirm
the real method for "is this profile a manager scoped to tenant X" (vs. the global `is_manager`
flag) — do not assume a method name.
**Where**: `core/src/use_cases/shared/audit/fetch_resource_audit_trail.rs` (+ `mod.rs`).
**Depends on**: T4
**Reuses**: `Profile::with_tenant_ownership_or_error`, `Profile::get_related_account_or_error`, `Profile::with_read_access`, `Profile::on_account` — no new authorization primitive.
**Requirement**: AUDIT-07

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] Staff profile → always allowed (verified by test).
- [ ] Tenant owner/manager profile for a tenant-scoped resource whose `tenant_id` matches → allowed; a tenant owner/manager for a *different* tenant → denied.
- [ ] Account-owner profile for a personal (non-tenant) resource whose owning account matches → allowed; an unrelated account owner → denied.
- [ ] A profile with none of the above standings → denied with `use_case_err(...)`.
- [ ] Gate check passes: `cargo test -p myc-core fetch_resource_audit_trail`.
- [ ] Test count: at least 5 tests pass (one per bullet above, staff/tenant-allow/tenant-deny/owner-allow/owner-deny).

**Tests**: unit
**Gate**: quick

**Commit**: `feat(core): add fetch_resource_audit_trail use case with permission rule`

---

### T7: Postgres adapter — registration + fetching repositories [P]

**What**: Implement `ResourceAuditLogRegistrationSqlDbRepository` (holds a plain, non-injected
`sender: mpsc::Sender<NewResourceAuditLogEvent>` field, `create()` does `try_send` per
`design.md`) and `ResourceAuditLogFetchingSqlDbRepository` (`list_by_resource`/`list_by_tenant`
against the T1 schema, using the two indexes as the query's `ORDER BY`/`WHERE` shape). Register
both as components in `adapters/diesel_postgres/src/repositories/mod.rs`'s `module! { components = [...] }` list.
**Where**: `adapters/diesel_postgres/src/repositories/resource_audit_log/{resource_audit_log_registration,resource_audit_log_fetching}.rs`, `adapters/diesel_postgres/src/models/resource_audit_log.rs` (Diesel `Queryable`/`Insertable` structs), `adapters/diesel_postgres/src/repositories/mod.rs` (add to `module!`).
**Depends on**: T1, T4
**Reuses**: `DieselDbPoolProvider`'s externally-parameterized `Component` pattern (`config.rs`) for the `sender` field; `WebHookFetchingSqlDbRepository`'s `#[shaku(inject)] db_config: Arc<dyn DbPoolProvider>` pattern for the fetching repo.
**Requirement**: AUDIT-05

**Tools**: MCP: NONE. Skill: NONE (consider `rust-analyzer-lsp` for navigating Diesel query-builder types if needed).

**Done when**:
- [ ] `create()` never blocks — enqueues via `try_send`, logs+swallows `Full`/`Closed`, always returns `Ok(())`.
- [ ] `list_by_resource` issues `WHERE resource_id = ? ORDER BY created_at DESC`; `list_by_tenant` issues `WHERE tenant_id = ? ORDER BY created_at DESC`.
- [ ] Both components appear in `SqlAppModule`'s `components = [...]` list.
- [ ] Unit test for `create()` using a real bounded channel (not a mock) asserting the sent event matches the input, and asserting a full channel doesn't panic/error out.
- [ ] Gate check passes: `cargo build -p mycelium-diesel` and `cargo test -p mycelium-diesel resource_audit_log`.

**Tests**: unit (registration repo's channel behavior only — fetching repo query correctness relies on the same "adapters: low coverage, integration-relied-upon" convention as the rest of this crate; verify `list_by_resource`/`list_by_tenant` manually against dev Postgres, see Verify).
**Gate**: quick + build

**Verify**:
```bash
cargo test -p mycelium-diesel resource_audit_log
# manual: insert 3 rows for one resource_id via T1's psql snippet, call list_by_resource, confirm 3 rows newest-first
```

**Commit**: `feat(diesel): add resource audit log repositories`

---

### T8: SQLite adapter — registration + fetching repositories [P]

**What**: Same as T7, targeting `adapters/diesel_sqlite`, against T2's schema.
**Where**: `adapters/diesel_sqlite/src/repositories/resource_audit_log/{...}.rs`, `adapters/diesel_sqlite/src/models/resource_audit_log.rs`, `adapters/diesel_sqlite/src/repositories/mod.rs` (add to its `module!`).
**Depends on**: T2, T4
**Reuses**: Same patterns as T7, mirrored for the SQLite backend's existing conventions.
**Requirement**: AUDIT-06

**Tools**: MCP: NONE. Skill: NONE.

**Done when**: Same bullets as T7, adapted to SQLite; gate: `cargo build -p mycelium-diesel-sqlite` and `cargo test -p mycelium-diesel-sqlite resource_audit_log`.

**Tests**: unit
**Gate**: quick + build

**Commit**: `feat(diesel-sqlite): add resource audit log repositories`

---

### T9: `resource_audit_log_dispatcher` background consumer

**What**: Implement the single-consumer dispatcher exactly as specified in `design.md` — resolves
`&dyn DbPoolProvider` via `app_modules.resolve_ref()`, loops on `receiver.recv().await`, inserts
synchronously inline (no `spawn_blocking`, matching the rest of the codebase), `tracing::error!` +
`continue` on a failed insert, never panics on a business error.
**Where**: `ports/api/src/dispatchers/resource_audit_log_dispatcher.rs`.
**Depends on**: T7 (needs the Postgres repo/model shape to build the same insert this dispatcher issues directly — confirm whether the dispatcher reuses the adapter's `Insertable` model or defines its own; prefer reusing T7/T8's model type).
**Reuses**: `webhook_dispatcher.rs`'s structural pattern (`tokio::spawn`, `resolve_ref`, per-item try/continue error handling).
**Requirement**: AUDIT-05

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] Function signature: `pub(crate) async fn resource_audit_log_dispatcher(app_modules: Arc<SqlAppModule>, mut receiver: mpsc::Receiver<NewResourceAuditLogEvent>)`.
- [ ] Loop never exits on a single failed insert.
- [ ] Gate check passes: `cargo build -p mycelium-api`.

**Tests**: none (background task, integration-level — matches `webhook_dispatcher`'s own untested status per `TESTING.md`'s "Middleware not explicitly tested" note).
**Gate**: build

**Verify**: Manually enqueue an event via a test binary or temporary log line, confirm a row lands in `resource_audit_log` within one poll cycle; kill the DB connection mid-loop and confirm the task keeps running (logs the error, doesn't crash).

**Commit**: `feat(api): add resource_audit_log_dispatcher background consumer`

---

### T10: Wire channel + component parameters + dispatcher spawn in `main.rs`

**What**: In both `initialize_modules` branches (`postgres-backend` and `standalone`), create
`let (tx, rx) = tokio::sync::mpsc::channel::<NewResourceAuditLogEvent>(2048);` before
`SqlAppModule::builder()`, feed `tx` into
`.with_component_parameters::<ResourceAuditLogRegistrationSqlDbRepository>(...Parameters { sender: tx })`,
and add a `resource_audit_log_dispatcher(sql_module.clone(), rx).instrument(span.to_owned()).await;`
call alongside the existing `webhook_dispatcher(...)` line.
**Where**: `ports/api/src/main.rs` (both `#[cfg(feature = "postgres-backend")]` and `#[cfg(feature = "standalone")]` code paths).
**Depends on**: T7, T8, T9
**Reuses**: The exact `DieselDbPoolProvider` `.with_component_parameters` call site as the template (same file, ~20 lines away).
**Requirement**: AUDIT-05, AUDIT-06

**Tools**: MCP: NONE. Skill: NONE.

**Done when**:
- [ ] `cargo build -p mycelium-api` (default `postgres-backend`) succeeds.
- [ ] `cargo build -p mycelium-api --no-default-features --features standalone` succeeds.
- [ ] Booting the server with `SETTINGS_PATH=...` locally shows the dispatcher's startup span in logs.

**Tests**: none.
**Gate**: build (both feature sets — this is the task that proves AUDIT-06's "standalone build keeps compiling" success criterion)

**Commit**: `feat(api): wire resource audit log channel and dispatcher for both backends`

---

### T11: REST/RPC read endpoint

**What**: Expose `fetch_resource_audit_trail` (T6) over REST and RPC. Before writing the handler,
open the router/handler module for an existing `role_scoped` read endpoint of comparable shape
(e.g., a `*_fetching` handler) to confirm this codebase's actual path-naming and response-mapping
convention — do not invent a route path. Handler resolves `Profile` from the request exactly like
every other authenticated endpoint, resolves `&dyn ResourceAuditLogFetching` via
`app_modules.resolve_ref()`, and maps the result with the existing `create_response_kind`/
`handle_mapped_error` helpers.
**Where**: New handler file under `ports/api/src/rest/` (exact module TBD, confirmed against router conventions), plus its RPC counterpart, plus an `ApiDoc`/OpenAPI entry if that's required for every REST handler in this codebase (confirm against a recent handler addition).
**Depends on**: T6, T10
**Reuses**: An existing `role_scoped` fetching handler as the direct template for request parsing, `Profile` extraction, and response mapping.
**Requirement**: AUDIT-08

**Tools**: MCP: NONE. Skill: `myc-rpc` (for the RPC-side wrapper) if useful once the RPC method is registered.

**Done when**:
- [ ] REST handler accepts `resource_type` + `resource_id` (or `tenant_id` for the tenant-wide view) and returns the trail ordered newest-first.
- [ ] Unauthenticated / insufficiently-privileged requests get the same error shape as other permission failures.
- [ ] Gate check passes: `cargo build -p mycelium-api`.
- [ ] Manual verification: `curl` the endpoint as staff (200 + rows) and as an unrelated profile (403-equivalent).

**Tests**: none at this layer (matches `TESTING.md`'s "API endpoint handlers... lack inline tests" note) — correctness is covered by T6's use-case-level permission tests plus manual verification.
**Gate**: build

**Commit**: `feat(api): expose resource audit trail over REST/RPC`

---

### Instrumentation task template (applies to every T12+ row below)

**What**: In the named use case, after the primary write succeeds (never on a validation/error
path), call `emit_resource_audit_event(audit_repo, resource_type, resource_id, tenant_id, event, performed_by, metadata)`. Thread a new `audit_repo: Box<&dyn ResourceAuditLogRegistration>` parameter through the use case's signature and every REST/RPC/MCP caller that invokes it (resolve via `app_modules.resolve_ref()` in each caller, same as every other injected repo).
**Depends on**: T5, T10, and the previous task in the same domain lane (shared handler-file risk).
**Reuses**: `emit_resource_audit_event` (T5); the use case's existing `Profile`/`WrittenBy` construction (already present at most of these call sites, since it's how `created_by`/`updated_by` gets built today).
**Tools**: MCP: NONE. Skill: NONE.

**Done when** (every row below):
- [ ] The use case emits exactly one audit event on success, zero on failure/validation-reject.
- [ ] `resource_type`/`event` match the row; `tenant_id` is populated per the "tenant_id source" column (or left `None` where the column says so).
- [ ] Unit test (extends the use case's existing `#[cfg(test)]` module, or adds one) with a `mockall`-mocked `ResourceAuditLogRegistration` asserting `create` is called once with the right `resource_type`/`event`/`resource_id` on success, and not called when the use case returns an error before the write.
- [ ] Every REST/RPC/MCP caller updated to resolve and pass the new port; `cargo build --workspace` still succeeds.
- [ ] Gate check passes: `cargo test -p myc-core <function_name>`.

**Tests**: unit
**Gate**: quick

---

#### Account lane (P1) — T12 → T28, Requirement AUDIT-09

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T12 | `super_users/managers/account/create_system_account.rs` | `create_system_account` | created | `account.tenant_id` (usually `None`) |
| T13 | `super_users/staff/account/create_seed_staff_account.rs` | `create_seed_staff_account` | created | `None` |
| T14 | `super_users/staff/account/upgrade_account_privileges.rs` | `upgrade_account_privileges` | updated | `account.tenant_id` |
| T15 | `super_users/staff/account/downgrade_account_privileges.rs` | `downgrade_account_privileges` | updated | `account.tenant_id` |
| T16 | `role_scoped/users_manager/account/change_account_archival_status.rs` | `change_account_archival_status` | updated | `account.tenant_id` |
| T17 | `role_scoped/users_manager/account/change_account_approval_status.rs` | `change_account_approval_status` | updated | `account.tenant_id` |
| T18 | `role_scoped/users_manager/account/change_account_activation_status.rs` | `change_account_activation_status` | updated | `account.tenant_id` |
| T19 | `role_scoped/beginner/account/create_account_from_existing_user.rs` | `create_user_account` | created | `None` (beginner accounts are personal) |
| T20 | `role_scoped/beginner/account/delete_my_account.rs` | `delete_my_account` | deleted | `None` |
| T21 | `role_scoped/beginner/account/update_own_account_name.rs` | `update_own_account_name` | updated | `None` |
| T22 | `role_scoped/tenant_owner/account/create_management_account.rs` | `create_management_account` | created | `account.tenant_id` |
| T23 | `role_scoped/tenant_owner/account/delete_tenant_manager_account.rs` | `delete_tenant_manager_account` | deleted | `account.tenant_id` |
| T24 | `role_scoped/subscriptions_manager/account/create_subscription_account.rs` | `create_subscription_account` | created | `account.tenant_id` |
| T25 | `role_scoped/subscriptions_manager/account/create_role_associated_account.rs` | `create_role_associated_account` | created | `account.tenant_id` |
| T26 | `role_scoped/subscriptions_manager/account/update_account_name_and_flags.rs` | `update_account_name_and_flags` | updated | `account.tenant_id` |
| T27 | `role_scoped/tenant_manager/account/create_subscription_manager_account.rs` | `create_subscription_manager_account` | created | `account.tenant_id` |
| T28 | `role_scoped/tenant_manager/account/delete_subscription_account.rs` | `delete_subscription_account` | deleted | `account.tenant_id` |

All paths relative to `core/src/use_cases/`. `resource_id` = the account's own `id` in every row.

---

#### Tenant lane (P2) — T29 → T36, Requirement AUDIT-10

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T29 | `super_users/managers/tenant/create_tenant.rs` | `create_tenant` | created | the tenant's own id |
| T30 | `super_users/managers/tenant/delete_tenant.rs` | `delete_tenant` | deleted | the tenant's own id |
| T31 | `super_users/managers/tenant/include_tenant_owner.rs` | `include_tenant_owner` | updated | the tenant's own id |
| T32 | `super_users/managers/tenant/exclude_tenant_owner.rs` | `exclude_tenant_owner` | updated | the tenant's own id |
| T33 | `role_scoped/tenant_owner/tenant/update_tenant_name_and_description.rs` | `update_tenant_name_and_description` | updated | the tenant's own id |
| T34 | `role_scoped/tenant_owner/tenant/update_tenant_archiving_status.rs` | `update_tenant_archiving_status` | updated | the tenant's own id |
| T35 | `role_scoped/tenant_owner/tenant/update_tenant_trashing_status.rs` | `update_tenant_trashing_status` | updated | the tenant's own id |
| T36 | `role_scoped/tenant_owner/tenant/update_tenant_verifying_status.rs` | `update_tenant_verifying_status` | updated | the tenant's own id |

`resource_id` = the tenant's own `id` in every row (so `resource_id == tenant_id` here, unlike the account lane).

---

#### Guest-role lane (P2) — T37 → T42, Requirement AUDIT-11

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T37 | `role_scoped/guest_manager/guest_role/create_guest_role.rs` | `create_guest_role` | created | `None` — confirmed no tenant column on `guest_role` (schema.rs) |
| T38 | `role_scoped/guest_manager/guest_role/delete_guest_role.rs` | `delete_guest_role` | deleted | `None` |
| T39 | `role_scoped/guest_manager/guest_role/update_guest_role_name_and_description.rs` | `update_guest_role_name_and_description` | updated | `None` |
| T40 | `role_scoped/guest_manager/guest_role/update_guest_role_permissions.rs` | `update_guest_role_permission` | updated | `None` |
| T41 | `role_scoped/guest_manager/guest_role/insert_role_child.rs` | `insert_role_child` | updated | `None` |
| T42 | `role_scoped/guest_manager/guest_role/remove_role_child.rs` | `remove_role_child` | updated | `None` |

Because `guest_role` has no tenant linkage, its read permission (per T6) falls through to the
account-owner branch or staff-only — confirm during T37 whether a guest role has a natural
"owner" account at all; if not, these six rows are staff-only reads, matching the `webhook` posture.

---

#### User lane (P3) — T43 → T44, Requirement AUDIT-12

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T43 | `role_scoped/beginner/user/create_default_user/mod.rs` | `create_default_user` | created | inherited from the owning account's `tenant_id` |
| T44 | `role_scoped/beginner/user/delete_default_user.rs` | `delete_default_user` | deleted | inherited from the owning account's `tenant_id` |

---

#### Webhook lane (P3) — T45 → T47, Requirement AUDIT-12

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T45 | `role_scoped/system_manager/webhook/register_webhook.rs` | `register_webhook` | created | `None` — webhooks are global |
| T46 | `role_scoped/system_manager/webhook/update_webhook.rs` | `update_webhook` | updated | `None` |
| T47 | `role_scoped/system_manager/webhook/delete_webhook.rs` | `delete_webhook` | deleted | `None` |

---

#### Account-meta lane (P3) — T48 → T50, Requirement AUDIT-12

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T48 | `role_scoped/beginner/meta/create_account_meta.rs` | `create_account_meta` | created | inherited from the owning account's `tenant_id` |
| T49 | `role_scoped/beginner/meta/update_account_meta.rs` | `update_account_meta` | updated | inherited from the owning account's `tenant_id` |
| T50 | `role_scoped/beginner/meta/delete_account_meta.rs` | `delete_account_meta` | deleted | inherited from the owning account's `tenant_id` |

`resource_id` = the owning account's `id` (the meta key itself lives in `metadata`, not as its own UUID).

---

#### Tenant-meta lane (P3) — T51 → T52, Requirement AUDIT-12

| Task | File | Function | Event | tenant_id source |
| --- | --- | --- | --- | --- |
| T51 | `role_scoped/tenant_owner/meta/create_tenant_meta.rs` | `create_tenant_meta` | created | the tenant's own id |
| T52 | `role_scoped/tenant_owner/meta/delete_tenant_meta.rs` | `delete_tenant_meta` | deleted | the tenant's own id |

`resource_id` = the tenant's own `id`.

---

## Parallel Execution Map

```
Phase 1 (parallel):
  T1 [P] ── T2 [P] ── T3 [P]          (no shared files, all independent)

Phase 1 → 1.5 (sequential, needs T3):
  T3 ──→ T4 ──→ { T5, T6 }             (T5, T6 both only need T4; they touch different
                                          files — core/use_cases/shared/audit/{emit_*, fetch_*} —
                                          so T5 and T6 ARE parallel-safe with each other)

Phase 2 (parallel adapters, then sequential wiring):
  { T1,T4 } ──→ T7 [P] ─┐
  { T2,T4 } ──→ T8 [P] ─┼──→ T9 ──→ T10 ──→ T11
                          (T9 needs T7 for the model type; T10 needs T7+T8+T9; T11 needs T6+T10)

Phase 3 (7 parallel lanes, sequential inside each lane):
  { T5,T10 } ──┬── Account:      T12→T13→...→T28
               ├── Tenant:       T29→T30→...→T36
               ├── Guest-role:   T37→...→T42
               ├── User:         T43→T44
               ├── Webhook:      T45→...→T47
               ├── Account-meta: T48→...→T50
               └── Tenant-meta:  T51→T52
```

**Parallelism constraint reminder**: within any lane, tasks are NOT `[P]` relative to each other
(shared handler-file risk) — only the 7 lanes are parallel relative to each other, and only T1/T2/T3
and T7/T8 are parallel within Phase 1/2.

---

## Task Granularity Check

| Task | Scope | Status |
| --- | --- | --- |
| T1–T2 | 1 migration each | ✅ Granular |
| T3 | 4 tiny DTO files, one cohesive concept (audit event shape) | ✅ Granular (2-3 related things in one commit, cohesive) |
| T4 | 2 trait files, one cohesive port pair | ✅ Granular |
| T5, T6 | 1 function each | ✅ Granular |
| T7, T8 | 2 repos + 1 model + 1 module-list edit, one cohesive adapter slice per backend | ✅ Granular (cohesive; splitting registration/fetching into separate tasks would force two edits to the same `module!` list from two different agents — higher conflict risk than value) |
| T9 | 1 background task | ✅ Granular |
| T10 | 1 file, 2 cfg-gated blocks, one cohesive wiring change | ✅ Granular |
| T11 | 1 REST handler + 1 RPC wrapper, one cohesive endpoint | ✅ Granular |
| T12–T52 | 1 use case (+ its callers) each | ✅ Granular |

---

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| --- | --- | --- | --- |
| T1 | None | None | ✅ Match |
| T2 | None | None | ✅ Match |
| T3 | None | None | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T4 | T4 → T5 | ✅ Match |
| T6 | T4 | T4 → T6 | ✅ Match |
| T7 | T1, T4 | T1, T4 → T7 | ✅ Match |
| T8 | T2, T4 | T2, T4 → T8 | ✅ Match |
| T9 | T7 | T7 → T9 | ✅ Match |
| T10 | T7, T8, T9 | T7, T8, T9 → T10 | ✅ Match |
| T11 | T6, T10 | T6, T10 → T11 | ✅ Match |
| T12 (lane head) | T5, T10 | T5, T10 → Account lane | ✅ Match |
| T13–T28 | T5, T10, previous lane task | shown as sequential arrows within Account lane | ✅ Match |
| T29 (lane head) | T5, T10 | T5, T10 → Tenant lane | ✅ Match |
| T30–T36 | T5, T10, previous lane task | sequential within Tenant lane | ✅ Match |
| T37 (lane head) | T5, T10 | T5, T10 → Guest-role lane | ✅ Match |
| T38–T42 | T5, T10, previous lane task | sequential within Guest-role lane | ✅ Match |
| T43 (lane head) | T5, T10 | T5, T10 → User lane | ✅ Match |
| T44 | T5, T10, T43 | sequential within User lane | ✅ Match |
| T45 (lane head) | T5, T10 | T5, T10 → Webhook lane | ✅ Match |
| T46–T47 | T5, T10, previous lane task | sequential within Webhook lane | ✅ Match |
| T48 (lane head) | T5, T10 | T5, T10 → Account-meta lane | ✅ Match |
| T49–T50 | T5, T10, previous lane task | sequential within Account-meta lane | ✅ Match |
| T51 (lane head) | T5, T10 | T5, T10 → Tenant-meta lane | ✅ Match |
| T52 | T5, T10, T51 | sequential within Tenant-meta lane | ✅ Match |

---

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| --- | --- | --- | --- | --- |
| T1, T2 | SQL migration | none (no Rust code) | none | ✅ OK |
| T3 | Core DTOs | Core has "Medium" coverage; new DTOs get round-trip tests | unit | ✅ OK |
| T4 | Core port traits (no logic) | none — nothing executable to test | none | ✅ OK |
| T5, T6 | Core use cases | Core "Medium" — new logic gets unit tests | unit | ✅ OK |
| T7, T8 | Adapters | Adapters "Low" — existing convention relies on integration/manual verification; registration repo's non-blocking behavior IS unit-testable and is tested | unit (registration only) + manual verify (fetching) | ✅ OK |
| T9 | Dispatcher (ports/api background task) | Matches `webhook_dispatcher`'s own untested status (TESTING.md: "Middleware not explicitly tested") | none | ✅ OK |
| T10 | `main.rs` wiring | Ports "Low"; this is glue code, proven by the build gate on both feature sets | none | ✅ OK |
| T11 | REST/RPC handler | TESTING.md: "API endpoint handlers... lack inline tests" — existing convention | none (permission logic already covered by T6's unit tests) | ✅ OK |
| T12–T52 | Core use cases (instrumentation) | Core "Medium" — each gets a unit test asserting the audit call happens/doesn't happen | unit | ✅ OK |

No `Tests: none` row above is deferral — each maps to an existing, documented coverage-matrix
gap (adapters/ports are already "Low" in this codebase) rather than skipping a test the matrix
requires.

---

## Tools

Defaulting to **no MCP and no skill** for the foundation/adapter/wiring tasks (T1–T11) — this is
plain Rust/Diesel work with no external lookup needed beyond the codebase itself. Two exceptions
called out inline: `rust-analyzer-lsp` is available if Diesel's query-builder types get hard to
navigate in T7/T8, and the `myc-rpc` skill is available for T11's RPC wrapper once the method is
registered. Instrumentation tasks (T12–T52) also default to no MCP/skill — they're mechanical
repetitions of the T5 pattern. Say if you'd like a different tool assigned to any task before
Execute starts.
