# State

**Last Updated:** 2026-04-18
**Current Work:** M1 — Stability & Safety (Forwarded header done; JWT secret validation, middleware tests, mTLS pending)

---

## Recent Decisions (Last 60 days)

### AD-001: Use `OnceLock<Result<Tera, String>>` instead of `lazy_static! + panic!` (2026-04-06)

**Decision:** Replace the `lazy_static!` Tera initialization (which called `panic!` on failure) with
`std::sync::OnceLock<Result<Tera, String>>`, initialized lazily and propagating errors to callers.

**Reason:** `OnceLock` is available since Rust 1.70 (already required by this crate), supports
fallible init, and avoids the `lazy_static` dependency pattern. `Tera::default()` + runtime error
was considered but rejected — it hides the init failure too silently.

**Trade-off:** Callers of the template accessor must now handle `Result`; slightly more boilerplate
at call sites.

**Impact:** All template-render call sites must propagate errors via `?` or explicit match.

---

### AD-002: Propagate `choose_host()` error at call sites (2026-04-06)

**Decision:** Changed `choose_host()` signature to return `Result<String, MappedErrors>` and updated
both call sites (`route.rs`, `load_operations_from_downstream_services.rs`) to use `?`.

**Reason:** The change to `service.rs` forced a signature change; updating call sites was mandatory,
not optional. Committed all 5 files together as one atomic change.

**Trade-off:** None — this was the only correct approach.

**Impact:** Any future call site adding `choose_host()` must handle the error.

---

## Active Blockers

_(none)_

---

## RPC ↔ REST Audit (2026-04-13)

Full audit of all 12 RPC dispatcher files (88 methods total) against their REST equivalents.
REST is the reference — it is validated; RPC is what may diverge.

### Fixed

**`beginners.accounts.create` (`BEGINNERS_ACCOUNTS_CREATE`)** — resolved in this session.

- REST `create_default_account_url` does not use `MyceliumProfileData` extractor; it calls
  `check_credentials_with_multi_identity_provider` directly from `req`.
- RPC `admin_jsonrpc_post` was extracting `profile: MyceliumProfileData` as an Actix extractor,
  which returned HTTP 403 before the handler body ran for users with a valid JWT but no account.
- Fix: profile extraction moved inside the handler body; `GatewayError::Forbidden` falls back to
  an anonymous profile (struct literal with `Uuid::nil()`), allowing the dispatcher to be reached.
  The dispatcher already re-validates credentials independently.
- File changed: `ports/api/src/rpc/handlers.rs` only.

### Remaining divergences

_(none — all resolved)_

**`service.listDiscoverableServices`** — resolved in this session.

- REST `GET /services/tools` is fully public (`security(())`; no `MyceliumProfileData`).
- RPC was blocking unauthenticated callers with `GatewayError::Unauthorized` before the dispatcher
  ran.
- Fix: added `GatewayError::Unauthorized(_)` alongside `GatewayError::Forbidden(_)` in the
  anonymous-profile fallback in `admin_jsonrpc_post`. Both now fall through to the dispatcher.
  Protected methods remain secure via internal dispatcher checks (`profile.acc_id`, `is_manager`,
  `is_staff`, etc.).

**Decision:** RPC must mirror REST visibility. If a REST endpoint is public, the equivalent RPC
method must also be reachable without authentication.

### Clean scopes (no divergences)

`managers`, `accountManager`, `guestManager`, `subscriptionsManager`, `systemManager`,
`tenantManager`, `tenantOwner`, `usersManager`, `staff`, `gatewayManager`, `service` — all 88
methods have consistent profile requirements, credential extraction patterns, and authorization
checks between RPC and REST.

---

## Lessons Learned

### L-001: Signature changes in domain DTOs ripple to call sites outside the feature scope (2026-04-06)

**Context:** The `fix-notifier-panics` spec listed 3 target files. Changing `choose_host()` to
return `Result` forced updates in `route.rs` and `load_operations_from_downstream_services.rs`,
which were not in the spec.

**Problem:** Spec scope was defined by panic sites, not by the full call graph of changed APIs.

**Solution:** Committed all 5 files together; spec traceability remained valid since the call-site
changes were mechanical (add `?`), not behavioral.

**Prevents:** Future specs that change a public DTO method should proactively grep call sites and
include them in scope.

---

## Quick Tasks Completed

| #   | Description                             | Date       | Commit       | Status  |
| --- | --------------------------------------- | ---------- | ------------ | ------- |
| 001 | fix-notifier-panics (medium)            | 2026-04-06 | `b41b381c`   | ✅ Done |
| 002 | RFC 7239 Forwarded header compliance    | 2026-04-18 | `6faa212f`   | ✅ Done |

---

## Current Focus

**M3 — Magic Link Auth ✅ Complete** — GT0–GT7 implemented. Spec updated to `Status: Implemented` (2026-04-18).

**M1 — Stability & Safety (in progress)**

| Item | Status |
|---|---|
| Panic elimination (notifier + boot) | ✅ Complete |
| RFC 7239 Forwarded header compliance | ✅ Complete (`6faa212f`) |
| JWT secret validation at startup | Planned |
| Router & auth middleware tests | Planned |
| mTLS client certificate auth | Planned |

### Implementation notes

- `MagicLinkTokenMeta` lives in `core/src/domain/dtos/token/token/magic_link_token.rs`
  (new submodule under `token/token/`), not in `meta/` (which only contains `UserRelatedMeta`)
- `verify_magic_link` use case returns `User` (not `(String, Duration)`) — JWT encoding
  happens in the REST handler, following the existing `check_email_password_validity` pattern
- The display token is invalidated with `jsonb_set(meta, '{token}', 'null'::jsonb)`
- `tera = "1"` added directly to `ports/api/Cargo.toml` (not in workspace deps)
- `verify_magic_link_url` uses `get_not_redacted_user_by_email` for fetching the user

## Deferred Ideas

- [x] Audit remaining `unwrap()` calls across the full codebase (test code excluded) — **Audited 2026-04-06**
  - Systemic issue: Diesel ORM layer uses `unwrap()` pervasively for type conversions from DB records
  - ~215 JSON serde (`from_value`/`to_value` on JSONB columns) — medium risk, panics on corrupt DB data
  - ~174 timestamp (`and_local_timezone(Local).unwrap()`) — low risk, only on DST ambiguity
  - ~47 DB string parse (`Uuid::from_str`, `Email::from_string`, `.parse()` on DB values) — medium risk
  - ~30 `Mutex::lock().unwrap()` — low risk, only on lock poisoning
  - ~99 static literal `from_str("...")` — zero risk, compile-time safe
  - ~8 SSL/startup fail-fast in `main.rs` — acceptable
  - Recommended: create a dedicated M1/M2 feature to harden the Diesel adapter layer with proper `?`-propagation
- [ ] Email address validation in the DTO layer (not just at send time) — Captured during: fix-notifier-panics
- [ ] Hot-reloading Tera templates (ops/config concern) — Captured during: fix-notifier-panics

---

## Todos

_(none)_

---

## Preferences

**Model Guidance Shown:** never
