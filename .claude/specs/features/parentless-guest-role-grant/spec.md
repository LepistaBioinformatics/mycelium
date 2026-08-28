# Feat: allow granting a parentless (root) guest role when the requester already holds it

**Status:** Implemented — gates pending, awaiting user UAT before commit
**Scope:** Medium (1 file changed + tests)
**Issue:** [LepistaBioinformatics/mycelium#187](https://github.com/LepistaBioinformatics/mycelium/issues/187)
**Branch:** `feat/parentless-guest-role-grant`

---

## Problem

`guest_to_children_account` (`core/src/use_cases/role_scoped/account_manager/guest/guest_to_children_account.rs`)
gates a grant on four rules:

1. the requester holds `accounts-manager` **with write** on the tenant + target account;
2. the granted role **has a parent** (`get_parent_by_child_id`);
3. the requester **holds that parent**;
4. the granted role is listed in `parent_role.children`.

Rule (2) makes a **root role undelegable by anyone**, because a root role has no parent. In
production the call fails at rule (2):

```
[codes=MYC00013 error_type=use-case-error]
Guest role parent not found: <role-id>
```

Working around it via data is impossible: `insert_role_child` requires
`parent.permission >= child.permission`, `get_or_create` de-duplicates on `(slug, permission)`,
and `Permission` has only `Read = 0` / `Write = 1`. A same-slug parent for a `(slug, write)` role
is therefore inexpressible, and a different-slug parent breaks downstream authorization —
`LicensedResource.role` carries the **slug** and consumers match it by equality.

---

## Requirements

| ID | Requirement |
|---|---|
| **R1** | When the target role **has a parent**, the existing rules (3) and (4) apply unchanged. |
| **R2** | When the target role is a **root** (no parent), the grant is allowed if — and only if — the requester **already holds the target role itself**. |
| **R3** | The root-path possession check runs against the profile **scoped to the tenant and the target account** (`on_tenant(tenant_id).on_account(target_account_id)`), not the full license list. Otherwise holding the role on one account would authorize granting it on another. |
| **R4** | Possession is matched by `role_id`, not by slug — same predicate the parent check already uses. `get_or_create` de-duplicates on `(slug, permission)`, so a `role_id` match already pins the permission. |
| **R5** | Rule (1) is untouched: `accounts-manager` with write on the tenant + target account is still required. |
| **R6** | The target role is resolved **before** the parent branch. A nonexistent `target_role_id` has no parent, so resolving the parent first would route a missing role into the root path and report it as "you do not hold this role" instead of "guest role not found". |
| **R7** | The root-path denial carries `MYC00013` + `with_exp_true()`, with a message distinct from the parent-path denial so the two branches are distinguishable in logs. |
| **R8** | The public signature of `guest_to_children_account` does not change — the three entry points (REST `guest_endpoints.rs`, the RPC dispatcher, openrpc) are unaffected. |
| **R9** | Documentation that encodes the old rule is corrected: the use-case doc comment, and the openrpc method description (`ports/api/src/rpc/openrpc/methods/account_manager.rs`), which described the target as "(child role)". The utoipa REST annotations never stated the rule and are untouched. |

---

## Decisions

**The parent-path possession check stays unscoped.** The issue's rule table reads
"target role has a parent → current rule", and rule (1) forces `accounts-manager` on the *target*
account but nothing guarantees the requester holds the *parent role* on that same account — in a
"guest to children account" topology they plausibly hold it on the managing account. Scoping it
would be a behavior change beyond the issue and could break live grants.

**Resulting asymmetry:** the root path is scoped to tenant + target account, the parent path is
not. Recorded deliberately; worth a follow-up ticket to align them (would require confirming the
production topology first).

**Staff and manager profiles stay strict.** `get_related_account_or_error` short-circuits for
`is_staff` / `is_manager` and returns before touching `licensed_resources`, so such a profile can
clear rule (1) with an empty license list and then fail the possession check. That is exactly
today's behavior on the parent path, and it preserves the invariant the issue states: **nobody
grants what they do not have.**

---

## Anti-escalation invariant

Preserved. The requester must hold, on the target account, either the parent of the granted role
(existing path) or the granted role itself (new path). No path lets a requester grant a role they
do not have.

The change also narrows an existing inconsistency: the `subscriptions-manager` path
(`guest_user_to_subscription_account`) applies **no hierarchy at all**, so the restriction fell
only on the less privileged `accounts-manager` path.

---

## Adapter contract — verified

The change reinterprets `get_parent_by_child_id` returning `NotFound` as "this role is a root".
Both backends agree on that semantics, so the feature is **not** Postgres-only:

| Adapter | No-parent result |
|---|---|
| `adapters/diesel_postgres/.../guest_role_fetching.rs:78` | `.optional()` → `Ok(FetchResponseKind::NotFound(Some(id)))` |
| `adapters/diesel_sqlite/.../guest_role_fetching.rs:67` | `.optional()` → `Ok(FetchResponseKind::NotFound(Some(id)))` |

Neither returns `Err(diesel::NotFound)` (which `?` would propagate before the branch is reached),
and neither synthesizes an empty `Found` parent (which would route a root role into the parent
path and fail on the `children` check). No other adapter implements `GuestRoleFetching` —
`mem_db`, `kv_db`, `moka_cache`, `postgres_kv`, `service`, `shared` and `notifier` do not.

This matters because SQLite backs Standalone Mode, which is shipped, and the seven tests all stub
the trait — they could not have caught a divergence.

---

## Documentation touched

Signature unchanged, so no entry point needed code changes. Two descriptions encoded the old
"child role" framing and were corrected:

- the use-case doc comment on `guest_to_children_account`;
- the openrpc method description in
  `ports/api/src/rpc/openrpc/methods/account_manager.rs`.

The utoipa `#[utoipa::path]` block in `guest_endpoints.rs` never stated the hierarchy rule —
untouched.

---

## Adapter contract — verified on both backends

The change reinterprets `get_parent_by_child_id` returning `NotFound` as "this role is a root".
That is a contract the use case now depends on, so both implementations were read:

| Adapter | No-parent case | |
|---|---|---|
| `adapters/diesel_postgres/.../guest_role_fetching.rs:78` | `.first(conn).optional()` → `Ok(FetchResponseKind::NotFound(Some(id)))` | ok |
| `adapters/diesel_sqlite/.../guest_role_fetching.rs:66` | `.first(conn).optional()` → `Ok(FetchResponseKind::NotFound(Some(id)))` | ok |

Both use `.optional()`, so the absent parent surfaces as `Ok(NotFound)` rather than
`Err(diesel::NotFound)` wrapped by `map_err`. Neither synthesizes an empty `Found` parent, which
would otherwise route a root role into the parent path and fail on the `children` check. No other
adapter implements `GuestRoleFetching`. **The feature therefore works identically on Postgres and
on SQLite (Standalone Mode).**

---

## Tests

`guest_to_children_account` had **no test coverage**. Added, in-file `#[cfg(test)]` per repo
convention (hand-rolled `Stub*` structs, `#[tokio::test]`):

| Case | Expected |
|---|---|
| root role, held by the requester on the target account | Ok |
| root role, not held | Err MYC00013 |
| root role held on a **different account** of the same tenant | Err MYC00013 (R3) |
| child role, parent held | Ok |
| child role, parent not held | Err MYC00013 |
| child role, parent held but target not in `parent.children` | Err MYC00013 |
| target role not found | Err MYC00013 "Guest role not found" (R6) |

---

## Out of scope

- **Client-side mirroring.** Checked: `mycelium-webapp` needs no change — `GuestRoleSelector`
  lists guest roles without filtering by hierarchy, so root roles were never hidden there.
- **The slug-equality SDK helper** described in the issue's *Related finding* — explicitly a
  separate ticket, outside this repository.
- **A third `Permission` level (`Admin = 2`).** Considered in the issue and rejected here:
  `Permission::from_i32` maps `_ => Read`, so any not-yet-updated service would silently read `2`
  as read-only.
