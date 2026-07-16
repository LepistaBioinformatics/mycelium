# Tenant-Scoped Webhook Dispatch Specification

## Problem Statement

Webhooks have no tenant association anywhere in the gateway's domain model. The `WebHook` DTO,
its Diesel model, and the `webhook` table carry no `tenant_id`; `WebHookFetching::list_by_trigger`
filters only by `trigger` + `is_active`; and `dispatch_webhooks` sends the payload to every match.
Consequence: when a subscription account is created (updated / deleted) under tenant *A*, the
`SubscriptionAccount*` event is delivered to **every** registered webhook of that trigger,
including those belonging to unrelated tenants. There is no isolation of webhook delivery between
tenants — a cross-tenant data-leak and correctness gap for anyone building per-tenant integrations.

## Goals

- [ ] A webhook can optionally belong to a tenant (`tenant_id: Option<Uuid>`); a webhook with no
      tenant (`NULL`) remains a global webhook that behaves exactly as today.
- [ ] `SubscriptionAccountCreated`/`Updated`/`Deleted` events are delivered only to webhooks whose
      `tenant_id` matches the affected subscription account's tenant, **plus** global (`NULL`)
      webhooks — never to webhooks bound to a different tenant.
- [ ] Tenant owners and tenant managers can register and manage webhooks scoped to their own
      tenant; staff retain full control (register global or any-tenant webhooks, manage all).
- [ ] Existing webhooks keep working with zero backfill — the new column is nullable and defaults
      to `NULL` (global), so nothing changes for already-registered webhooks.
- [ ] Both persistence backends ship the change: `myc_diesel` (Postgres) and `myc_diesel_sqlite`
      (standalone), so the `standalone` feature build keeps compiling and behaving identically.
- [ ] Two tables gain a nullable `tenant_id`: `webhook` (the registered hook's scope) **and**
      `webhook_execution` (the queued dispatch artifact's scoping tenant), mirroring how the
      existing `trigger` field is a real column on both tables — the async dispatcher must read the
      scope from the persisted artifact, not recompute it.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Tenant scoping for `UserAccountCreated/Updated/Deleted` triggers | Personal accounts have no `tenant_id` (see STATE.md L-002); these events are inherently global. User chose "só SubscriptionAccount*" for this feature. |
| Tenant scoping for any future/system triggers | Same — only the three `SubscriptionAccount*` triggers get tenant-aware dispatch in this iteration. |
| Making `tenant_id` mandatory / backfilling existing webhooks | User chose the nullable-coexistence model; global webhooks remain first-class. |
| Per-webhook secret re-keying by tenant | Secret handling (`get_or_provision_dek`, system DEK) is unchanged; scoping is about *delivery targeting*, not encryption. |
| A webapp screen to manage tenant webhooks | Gateway-only feature; ships REST + RPC API. Webapp is a separate module/spec. |
| Account-level (sub-tenant) webhook scoping | Only tenant granularity is requested. |

---

## Domain notes (established from the codebase, not assumed)

- **Webhook management today is `system_manager`-scoped**: use cases live in
  `core/src/use_cases/role_scoped/system_manager/webhook/` (`register_webhook`, `update_webhook`,
  `delete_webhook`, `list_webhooks`); endpoints in
  `ports/api/src/rest/role_scoped/system_manager/webhook_endpoints.rs` and the `system_manager` RPC
  dispatcher. This feature adds a **tenant-owner/manager** management surface alongside it.
- **The three `SubscriptionAccount*` dispatch sites** all carry a tenant-bearing account payload:
  `create_subscription_account.rs:135`, `update_account_name_and_flags.rs:173/326`,
  `delete_subscription_account.rs:105`, `propagate_existing_subscription_account.rs:95`. Each calls
  `register_webhook_dispatching_event(...)` with the account as payload — the account's
  `AccountType::Subscription { tenant_id }` is the tenant to scope on.
- **Dispatch is asynchronous and decoupled from registration**: `register_webhook_dispatching_event`
  persists a `WebHookPayloadArtifact` (stored in the `webhook_execution` table); the
  `webhook_dispatcher` background task later calls `dispatch_webhooks`, which calls `list_by_trigger`.
  Therefore the tenant to filter on must be **persisted on the artifact** at registration time — it
  cannot be recomputed at dispatch time without re-parsing the payload. This is the key plumbing
  decision, and it means **two** schema migrations: `webhook.tenant_id` (the hook's scope) and
  `webhook_execution.tenant_id` (the artifact's scoping tenant). Precedent: `trigger` is already a
  real column on both `webhook` and `webhook_execution` (`schema.rs:208`), not buried in the payload
  blob — `tenant_id` follows the same pattern.
- **`UserAccount*` sites are left untouched**: `create_account_from_existing_user.rs:181`,
  `update_own_account_name.rs:80`, `delete_my_account.rs:88` — these pass no tenant and continue to
  hit global + (by definition, none) tenant webhooks.

---

## User Stories

### P1: Tenant-isolated dispatch for subscription-account webhooks ⭐ MVP

**User Story**: As an operator building a per-tenant integration, I want the
`subscriptionAccount.*` webhook events for tenant *A* to reach only tenant *A*'s webhooks (and any
global webhooks), so that one tenant's account lifecycle is never leaked to another tenant's
endpoint.

**Why P1**: This is the actual reported defect and the vertical slice that exercises the whole
stack — schema (both backends), DTO, artifact plumbing, fetching filter, dispatch, and the
registration path that lets a webhook carry a tenant. Staff can set `tenant_id` on registration in
this slice; the self-service owner/manager surface is P2.

**Acceptance Criteria**:

1. WHEN a webhook is registered with a `tenant_id` THEN the system SHALL persist that `tenant_id`
   on the `webhook` row (both Postgres and SQLite backends).
2. WHEN a webhook is registered without a `tenant_id` THEN the row's `tenant_id` SHALL be `NULL`
   and the webhook SHALL behave as a global webhook (delivered for all tenants), preserving current
   behavior for every already-existing webhook.
3. WHEN a `SubscriptionAccountCreated`/`Updated`/`Deleted` event is registered for an account whose
   `AccountType` carries `tenant_id = T` THEN the persisted dispatch artifact SHALL record `T` as
   its scoping tenant.
4. WHEN the dispatcher processes a `SubscriptionAccount*` artifact scoped to tenant `T` THEN it
   SHALL deliver only to webhooks WHERE `trigger` matches AND (`tenant_id = T` OR `tenant_id IS
   NULL`) AND `is_active = true` — never to a webhook bound to a tenant other than `T`.
5. WHEN there are no matching webhooks (no tenant-`T` and no global) THEN the artifact SHALL be
   marked `Skipped`, exactly as the current no-match path does.
6. WHEN a `UserAccount*` (non-tenant-aware) event is dispatched THEN it SHALL deliver only to
   **global** webhooks (`tenant_id IS NULL`), NOT to any tenant-bound webhook. This is
   backward-identical today (every existing webhook is `NULL`), and it structurally prevents a
   tenant-bound webhook from receiving system-wide user-account events (a cross-tenant leak) once P2
   lets tenants register their own hooks. "Global-only for global triggers" is the safe default; the
   validation in WTS-13 is the belt-and-suspenders complement.

**Independent Test**: Register three webhooks for `subscriptionAccount.created` — one for tenant A,
one for tenant B, one global (NULL). Create a subscription account under tenant A. Assert the tenant-A
hook and the global hook receive the payload and the tenant-B hook does not. Repeat under `standalone`
(SQLite) build and see identical behavior.

---

### P2: Tenant owners/managers self-manage their tenant's webhooks

**User Story**: As a tenant owner or tenant manager, I want to register, list, update, and delete
webhooks bound to my own tenant without needing staff, so I can wire up my tenant's integrations
myself while remaining unable to touch other tenants' or global webhooks.

**Why P2**: Delivers the authorization model the user selected ("tenant owner/manager + staff"). It
is separable from P1 — P1 already isolates *delivery*; P2 adds the self-service *management* surface.

**Acceptance Criteria**:

1. WHEN a tenant owner or manager registers a webhook THEN the system SHALL force its `tenant_id` to
   the caller's tenant (the caller cannot register a global or other-tenant webhook).
2. WHEN a tenant owner/manager lists, updates, or deletes webhooks THEN the system SHALL operate only
   on webhooks whose `tenant_id` equals the caller's tenant; a webhook belonging to another tenant or
   a global webhook SHALL NOT be visible or mutable to them.
3. WHEN staff registers a webhook THEN staff SHALL be able to set `tenant_id` to `NULL` (global) or
   to any tenant, and SHALL be able to list/update/delete any webhook regardless of tenant.
4. WHEN a non-owner/non-manager/non-staff profile calls the tenant webhook endpoints THEN the system
   SHALL deny the request.
5. WHEN both a REST and an RPC path exist for these operations THEN they SHALL expose the same
   authorization and behavior (per the repo's REST-is-reference RPC-parity rule).
6. WHEN any caller (owner/manager or staff) registers a tenant-scoped webhook (`tenant_id` set) for a
   trigger that is not tenant-aware in this iteration (`UserAccount*`) THEN the system SHALL reject it
   with a clear validation error — a tenant-bound webhook on a global-only trigger would never fire
   (P1 AC6) and, absent this guard, would be a latent misconfiguration. This closes the leak surface
   opened by self-service registration and MUST ship in the same slice as P2.

**Independent Test**: As tenant-A owner, register a webhook (verify it is forced to tenant A), list
(see only tenant-A hooks), attempt to delete a tenant-B hook (denied). As staff, register a global
hook and delete a tenant-A hook (allowed).

---

### P3: Trigger/scope validation and observability

**User Story**: As an operator, I want the system to reject nonsensical tenant/trigger combinations
and to make the scoping visible, so misconfiguration fails loudly instead of silently never firing.

**Why P3**: Diagnostics; the feature is functional without them but more auditable with. (The
tenant/trigger validation guard was promoted to P2 — see P2 AC6 / WTS-13 — because it closes a leak
surface rather than being mere polish.)

**Acceptance Criteria**:

1. WHEN `dispatch_webhooks` runs THEN it SHALL log the scoping tenant (or "global") and the count of
   matched webhooks, so delivery targeting is auditable from traces.
2. WHEN the `WebHook` is serialized over the management API THEN `tenant_id` SHALL be included in the
   response so operators can see a webhook's scope.

---

## Edge Cases

- WHEN a subscription account's `AccountType` unexpectedly carries no `tenant_id` (data anomaly) THEN
  the artifact SHALL fall back to global-only delivery (treat as `NULL` scope) and log a warning —
  never panic, never leak to a wrong tenant.
- WHEN a tenant is deleted while webhooks bound to it still exist THEN those webhooks SHALL simply
  stop matching any future event (no cascade requirement in this spec); document, do not implement
  cleanup here.
- WHEN a webhook's `tenant_id` matches the event tenant AND a global webhook also matches THEN both
  SHALL receive the event (union, not exclusive-or) — the tenant hook does not suppress global hooks.
- WHEN the dispatch artifact was persisted before this feature shipped (no scoping tenant recorded)
  THEN it SHALL be treated as global scope on dispatch (backward-compatible drain of the queue).
- WHEN staff updates an existing global webhook to bind a tenant (or vice-versa) THEN the change SHALL
  take effect for subsequently-registered events only (already-queued artifacts keep their recorded
  scope).

---

## Requirement Traceability

| Requirement ID | Story | Area | Status |
| --- | --- | --- | --- |
| WTS-01 | P1 | Schema: nullable `tenant_id` on `webhook` **and** `webhook_execution` (Postgres, `myc_diesel`) | Pending |
| WTS-02 | P1 | Schema: nullable `tenant_id` on `webhook` **and** `webhook_execution` (SQLite, `myc_diesel_sqlite`) | Pending |
| WTS-03 | P1 | `WebHook` DTO gains `tenant_id: Option<Uuid>` (+ serialization) | Pending |
| WTS-04 | P1 | `WebHookPayloadArtifact` carries a scoping `tenant_id` (new field + `webhook_execution` column mapping), set at registration | Pending |
| WTS-05 | P1 | `register_webhook_dispatching_event` accepts + persists tenant; the five `SubscriptionAccount*` sites pass the account's tenant | Pending |
| WTS-06 | P1 | `WebHookFetching` tenant-aware fetch (`trigger` + (`tenant_id = T` OR `NULL`) for subscription triggers; global-only for non-tenant-aware); both adapters | Pending |
| WTS-07 | P1 | `dispatch_webhooks` reads the artifact's scoping tenant and applies the fetch rule per WTS-06 | Pending |
| WTS-08 | P1 | `register_webhook`/`WebHookRegistration` accept `tenant_id`; both adapters persist it | Pending |
| WTS-09 | P2 | Tenant-owner/manager webhook use cases (register/list/update/delete, tenant-forced) | Pending |
| WTS-10 | P2 | REST endpoints (tenant-owner scope) for tenant webhook management | Pending |
| WTS-11 | P2 | RPC methods (parity with REST) for tenant webhook management | Pending |
| WTS-12 | P2 | Authorization: owner/manager restricted to own tenant; staff unrestricted | Pending |
| WTS-13 | P2 | Validation: reject tenant-scoped webhook for non-tenant-aware triggers (leak guard; ships with P2) | Pending |
| WTS-14 | P3 | Dispatch observability (log scope + match count) + `tenant_id` in API responses | Pending |

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 14 requirements total, 0 mapped to tasks yet (Tasks phase pending), 0 unmapped.
Note: WTS-01/WTS-02 each cover **two** table migrations (`webhook` + `webhook_execution`) — the Tasks
phase should split them accordingly so neither column is dropped.

---

## Success Criteria

- [ ] A subscription-account event for tenant A reaches tenant-A webhooks + global webhooks only,
      verified end-to-end on both Postgres and SQLite (`standalone`) builds.
- [ ] Every pre-existing webhook (`tenant_id IS NULL`) behaves identically to before — no regression,
      no backfill required.
- [ ] Tenant owners/managers can fully self-manage their own tenant's webhooks and cannot see or
      mutate any other tenant's or global webhooks; staff can do everything.
- [ ] `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all`, and
      `cargo build -p mycelium-api --no-default-features --features standalone` all pass.
- [ ] REST and RPC tenant-webhook surfaces are behavior-identical (RPC-parity rule).
