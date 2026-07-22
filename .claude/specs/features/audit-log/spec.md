# Resource Audit Log Specification

## Problem Statement

The gateway has no durable, queryable trail of who created, changed, or removed a resource it
owns (accounts, tenants, guest roles, webhooks, ...). `account.created_by`/`updated_by` capture the
last writer on the row itself, but history is overwritten on every update and there is no way to
answer "what happened to this resource over time, and who did it." We need an immutable audit
trail, written without adding latency to the request path, readable only by the people who have a
legitimate stake in the resource (its owner, its managers, staff).

## Goals

- [ ] Every create/update/delete on an in-scope mycelium resource produces one immutable audit
      row: resource type, resource id, tenant id (when applicable), event kind, who did it, a
      timestamp captured at the moment of the event, and free-form JSON metadata.
- [ ] The write path never blocks or fails the triggering use case — the DB insert happens
      asynchronously via a channel + single background consumer, and audit failures are only
      traced, never propagated.
- [ ] The table is immutable in two independent layers: no `Updating`/`Deletion` port exists in
      code, and the database itself rejects `UPDATE`/`DELETE` via trigger.
- [ ] Reads are optimized for the dominant query — "show me the trail for resource X" — and are
      gated by role: staff always; tenant owners/managers for tenant-scoped resources; the
      resource's own account owner for personal resources.
- [ ] Both persistence backends (`myc_diesel` / Postgres and `myc_diesel_sqlite` / standalone) ship
      the port implementation, so the standalone build keeps compiling.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Downstream-service events (anything happening in services the gateway proxies to) | Explicitly excluded by the user — this audits mycelium itself only |
| `error_code` register/update/delete | Explicitly excluded by the user for this iteration |
| Auditing read/list operations | User asked for destructive activity only (create/update/delete) |
| Retention, rotation, or archival of audit rows | Not requested; a future feature once volume is known |
| A webapp screen to browse the audit trail | Out of scope for the gateway; this spec ships the API only |
| Route/service registration audit | These are config-file-driven in this codebase today (no DB-backed create/update/delete use case exists for them) |

---

## Resource Scope (P1 → P3, in implementation order)

"Subscription account" is not a separate table — it is an `account` row whose `account_type` JSON
tags it `Subscription { tenant_id }` (or `RoleAssociated`/`TenantManager`, which also carry a
`tenant_id`). So `resource_type = "account"` covers every account subtype; `tenant_id` on the audit
row is populated whenever the account's `AccountType` carries one, and stays `NULL` for
tenant-less accounts (staff, plain user, manager).

| resource_type | Covers | tenant_id populated when |
| --- | --- | --- |
| `account` | Every use case that creates/updates/deletes a row in `account`, regardless of `AccountType` (staff, manager, user, subscription, role-associated, tenant-manager, system) | `AccountType` carries a `tenant_id` |
| `account_meta` | Create/update/delete of a key in `account.meta` | Inherited from the owning account |
| `user` | Create/delete of the default user tied to an account | Inherited from the owning account |
| `tenant` | Create/delete/update of a tenant row, tenant owner include/exclude, archiving/trashing/verifying status | Always (it *is* the tenant) |
| `tenant_meta` | Create/delete of a key in `tenant.meta` | Always (inherited from the tenant) |
| `guest_role` | Create/update/delete of a guest role, permission changes, role-child insert/remove | `NULL` today — no tenant link found on `guest_role`; confirm during T-foundation, do not assume |
| `webhook` | Register/update/delete of a webhook | `NULL` — webhooks are global/system resources |

## User Stories

### P1: Audit trail for the `account` family ⭐ MVP

**User Story**: As a staff member or account owner, I want every create/update/delete on an
account to be recorded immutably, so that I can answer "what happened to this account, and who did
it" without relying on the row's own `created_by`/`updated_by`, which gets overwritten.

**Why P1**: `account` is the highest-cardinality, highest-risk resource (privilege escalation,
archival, activation all live here) and it exercises the full stack — schema, port, both adapters,
async write path, read endpoint, permission filter — end to end. Once this slice works, extending
to other resource types is largely mechanical repetition of the same pattern.

**Acceptance Criteria**:

1. WHEN any `account` create/update/delete use case (see Requirement Traceability) completes
   successfully THEN the system SHALL enqueue one audit event carrying `resource_type = "account"`,
   the account's id, its `tenant_id` (or `NULL`), the event kind, the actor (`WrittenBy`), a
   timestamp captured at that moment, and use-case-specific metadata.
2. WHEN the audit event is enqueued THEN the triggering use case SHALL return to its caller without
   waiting for the row to actually land in the database.
3. WHEN the background consumer inserts the row THEN it SHALL never mutate or delete an existing
   audit row, and a direct `UPDATE`/`DELETE` against the table via raw SQL SHALL be rejected by a
   database trigger.
4. WHEN a staff profile requests the audit trail for an account THEN the system SHALL return all
   matching rows ordered newest-first.
5. WHEN the account is tenant-scoped and the requesting profile is an owner or manager of that
   tenant THEN the system SHALL return the trail; WHEN the profile has none of staff/tenant
   owner/tenant manager/account-owner standing THEN the system SHALL deny the read.
6. WHEN the requesting profile is the account's own owner (personal, non-tenant account) THEN the
   system SHALL return the trail for that account.

**Independent Test**: Create an account, upgrade its privileges, archive it. Query the audit trail
as staff and see three ordered rows. Query as an unrelated account owner and get denied. Attempt a
raw `UPDATE resource_audit_log ...` and see it rejected by the trigger.

---

### P2: Tenant, subscription-account-via-tenant, and guest role domains

**User Story**: As a tenant owner/manager, I want audit trails for my tenant and the guest roles
and subscription accounts under it, so I can see who created/changed them.

**Why P2**: Extends the same pipeline to the tenant-scoped resources the user called out
explicitly ("subscription account connected to a tenant"), plus guest roles, which are the other
high-frequency mutation in the `tenant_manager`/`guest_manager` use-case families.

**Acceptance Criteria**:

1. WHEN a `tenant` create/update/delete use case completes THEN an audit event with
   `resource_type = "tenant"` and that tenant's own id as both `resource_id` and `tenant_id` SHALL
   be enqueued.
2. WHEN a `guest_role` create/update/delete use case completes THEN an audit event with
   `resource_type = "guest_role"` SHALL be enqueued (tenant linkage per the Resource Scope table's
   caveat — confirmed, not assumed, during implementation).
3. WHEN a tenant owner or tenant manager requests the audit trail for their tenant, or for a
   subscription account whose `tenant_id` matches, THEN the system SHALL return it.
4. WHEN a profile with no relationship to the tenant requests that trail THEN the system SHALL deny
   the read.

**Independent Test**: Create a tenant, add a subscription account under it, create a guest role.
Read the trail as the tenant owner and see all three creations; read as an unrelated tenant owner
and get denied.

---

### P3: `user`, `webhook`, and `*_meta` domains

**User Story**: As staff, I want the remaining in-scope resources (default users, webhooks,
account/tenant metadata) to also produce audit rows, so coverage matches "any destructive action on
a resource" without leaving gaps.

**Why P3**: Lower mutation frequency and lower blast radius than P1/P2; same mechanical pattern,
lowest priority to finish last.

**Acceptance Criteria**:

1. WHEN a `user`, `webhook`, `account_meta`, or `tenant_meta` create/update/delete use case
   completes THEN an audit event with the matching `resource_type` SHALL be enqueued, inheriting
   `tenant_id` from the owning account/tenant where applicable, or `NULL` for `webhook`.
2. WHEN staff requests any of these trails THEN the system SHALL return it; non-staff access
   follows the same owner/tenant rule as the parent resource.

**Independent Test**: Register a webhook, delete it. Read the trail as staff and see both rows; read
as a non-staff profile and get denied (webhooks have no owner/tenant).

---

## Edge Cases

- WHEN the background channel is full (consumer falling behind a burst) THEN the system SHALL drop
  the event, log a warning with the resource type/id, and SHALL NOT block or fail the caller.
- WHEN the database insert itself fails (e.g., DB unreachable) THEN the system SHALL log the error
  via `tracing` and continue the consumer loop — one failed insert must not stop subsequent events
  from being processed.
- WHEN the audited resource is later deleted THEN its audit rows SHALL remain (no cascading
  delete — `resource_id` is a plain UUID value, not a foreign key).
- WHEN a resource has no natural "owner" or tenant (e.g., `webhook`, a staff account) THEN only
  staff SHALL be able to read its trail.
- WHEN a requester has multiple qualifying standings (e.g., is both staff and the account owner)
  THEN the read SHALL succeed under the strongest applicable rule (staff short-circuits, matching
  the existing `Profile::get_related_account_or_error` precedent).
- WHEN a use case fails validation and never performs the write THEN no audit event SHALL be
  enqueued — only confirmed, successful mutations are recorded.

---

## Requirement Traceability

| Requirement ID | Story | Use case(s) | Status |
| --- | --- | --- | --- |
| AUDIT-01 | P1 | Schema: `resource_audit_log` table + trigger (Postgres) | Pending |
| AUDIT-02 | P1 | Schema: same table + trigger (SQLite/standalone) | Pending |
| AUDIT-03 | P1 | Core ports: `ResourceAuditLogRegistration`, `ResourceAuditLogFetching` + DTOs | Pending |
| AUDIT-04 | P1 | `emit_resource_audit_event` shared use-case helper | Pending |
| AUDIT-05 | P1 | Diesel/Postgres adapter + channel-backed dispatcher | Pending |
| AUDIT-06 | P1 | Diesel/SQLite adapter (parity, drains the same dispatcher pattern) | Pending |
| AUDIT-07 | P1 | Read use case with permission filter (staff / tenant owner-manager / account owner) | Pending |
| AUDIT-08 | P1 | REST/RPC read endpoint | Pending |
| AUDIT-09 | P1 | Instrument all `account` write use cases (see Design's use-case inventory) | Pending |
| AUDIT-10 | P2 | Instrument all `tenant` write use cases | Pending |
| AUDIT-11 | P2 | Instrument all `guest_role` write use cases | Pending |
| AUDIT-12 | P3 | Instrument `user`, `webhook`, `account_meta`, `tenant_meta` write use cases | Pending |

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 12 requirements total, all mapped to tasks in `tasks.md`, 0 unmapped.

---

## Success Criteria

- [ ] Every write use case listed in Design's inventory (except `error_code`) enqueues exactly one
      audit event on success and zero on failure/validation-reject.
- [ ] `cargo test --workspace --all` and `cargo build --workspace` pass for both the
      `full` and `standalone` feature sets.
- [ ] A direct `UPDATE`/`DELETE` against `resource_audit_log` fails with the trigger's error, on
      both backends.
- [ ] Reading a trail as staff always succeeds; as an unrelated profile always fails; as the
      resource's owner/tenant owner/tenant manager succeeds only when the relationship actually
      holds.
- [ ] No measurable added latency on the triggering request (the enqueue call is a non-blocking
      channel send).
