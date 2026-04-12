# Quick Task: Boot without downstream services configured

**Date:** 2026-04-11
**Scope:** Small — 2 files, 3 logic sites

---

## Context

Commit `6b805015` made `api.services` optional in the TOML config so the gateway
can start without downstream services declared. That change was limited to the
config model (`ports/api/src/models/api_config.rs`).

At boot time, `main.rs` calls `propagate_declared_roles_to_storage_engine`, which
queries the in-memory service DB. When no services are configured the DB is empty,
and two guards in the mem-db adapter return a hard error instead of `NotFound`.
This crashes the boot sequence with:

```
Error propagating declared roles to the SQL database:
[codes=none error_type=fetching-error] Routes already not initialized.
```

---

## Root Cause

Two independent bugs in the call chain:

### Bug 1 — `adapters/mem_db/src/repositories/service_read.rs`

Both `list_services` (line 106) and `list_services_paginated` (line 32) guard on
`db.len() == 0` and return a `fetching_err` instead of `NotFound`:

```rust
if db.len() == 0 {
    return fetching_err("Routes already not initialized.").as_error();
}
```

An empty DB is a valid state (no services configured). The correct response is
`Ok(FetchManyResponseKind::NotFound)`.

### Bug 2 — `core/src/use_cases/gateway/guest_roles/propagate_declared_roles_to_storage_engine.rs`

Line 37 treats `NotFound` from the repository as a fatal error:

```rust
_ => return use_case_err("No services found").as_error(),
```

When no services are configured there are no roles to propagate — this is a valid
no-op. The correct response is `return Ok(())`.

---

## Fix Plan

| # | File | Location | Change |
|---|---|---|---|
| 1 | `adapters/mem_db/src/repositories/service_read.rs` | `list_services_paginated`, line 32-33 | Replace `fetching_err(...).as_error()` with `Ok(FetchManyResponseKind::NotFound)` |
| 2 | `adapters/mem_db/src/repositories/service_read.rs` | `list_services`, line 106-107 | Same as above |
| 3 | `core/src/use_cases/gateway/guest_roles/propagate_declared_roles_to_storage_engine.rs` | `list_services` match arm, line 37 | Replace `use_case_err(...).as_error()` with `return Ok(())` |

`adapters/mem_db/src/repositories/routes_read.rs` has the same `db.len() == 0`
pattern (lines 34, 108, 189) but is not in the boot path. Out of scope for this
task — log in STATE.md as a known concern.

---

## Fix Plan — Addendum

A third call site was found after initial fix:

| # | File | Location | Change |
|---|---|---|---|
| 4 | `ports/api/src/openapi_processor/initialize_tools_registry.rs` | `list_services` match arm, line 50 | Replace `execution_err("Failed to fetch services")` with `vec![]` — no downstream services means tools registry initializes with gateway-only operations |

## Done When

- [x] Gateway boots successfully with `[api.services]` absent from config
- [x] `propagate_declared_roles_to_storage_engine` returns `Ok(())` when DB is empty
- [x] `initialize_tools_registry` skips downstream services gracefully when none configured
- [x] Unit tests for empty-DB paths in `service_read` and `propagate_declared_roles`
- [x] `cargo build --workspace` passes
- [x] `cargo test --workspace` passes
- [x] `cargo fmt --all -- --check` passes
