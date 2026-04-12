# Summary: Boot without downstream services configured

**Date:** 2026-04-11
**Status:** Complete

## Root cause

Three independent call sites treated an empty service DB as a fatal error
instead of a valid "no services configured" state.

## Changes

### `adapters/mem_db/src/repositories/service_read.rs`
- `list_services_paginated`: empty DB now returns `Ok(NotFound)` instead of `fetching_err`
- `list_services`: same fix
- Unit tests added for both empty-DB paths
- `tokio` added as dev-dependency

### `core/src/use_cases/gateway/guest_roles/propagate_declared_roles_to_storage_engine.rs`
- `NotFound` from repository now returns `Ok(())` — no services means no roles to propagate
- Unit test added (with `UnreachableGuestRoleRegistration` stub to assert `get_or_create` is never called)

### `ports/api/src/openapi_processor/initialize_tools_registry.rs`
- `NotFound` from `list_services` now yields an empty `services` vec instead of `execution_err`
- Tools registry proceeds to load gateway-only operations normally

### `adapters/mem_db/src/repositories/routes_read.rs`
- `match_single_path_or_error`: empty DB returns `Ok(NotFound(None))` instead of `fetching_err`
- `list_routes_paginated`: empty DB returns `Ok(NotFound)` instead of `fetching_err`
- `list_routes`: empty DB returns `Ok(NotFound)` instead of `fetching_err`
- These were initially deferred as "not in the boot path" but turned out to be triggered by
  `load_paths_from_spec` → `match_forward_address` during tools registry initialization

## Commits
- `b6a0d2c2` — fix(boot): support starting gateway without downstream services configured
- `8404c9ee` — fix(boot): skip downstream services in tools registry when none configured
- `cee201b3` — fix(boot): return NotFound instead of error in routes_read when DB is empty

## Gate checks
- `cargo fmt --all -- --check` ✓
- `cargo build --workspace` ✓
- `cargo test --workspace --all` ✓
