# Fix Notifier Panics — Specification

**Created:** 2026-04-03
**Scope:** Medium — 3 files, error-handling only, no architectural changes

## Problem Statement

Three locations in production code use `.unwrap()` or `panic!()` where a recoverable error
should be returned instead. Any of these can silently crash the running process:

1. **`adapters/notifier/src/repositories/remote_message_sending.rs:42,44,48`** — email address
   parsing panics on malformed input; the message body builder also panics if lettre rejects
   the assembled message. Invalid emails in the DB or a misconfigured sender crashes the notifier.

2. **`core/src/settings.rs:31`** — `lazy_static!` Tera template initialization calls `panic!()`
   if the templates directory is missing, unreadable, or contains a malformed template.
   This crashes the process at startup (or on first template access) with no graceful fallback.

3. **`core/src/domain/dtos/service.rs:90`** — `.choose(&mut rand::thread_rng()).unwrap()` panics
   when a service has an empty `hosts` list. A misconfigured service takes down request routing.

## Goals

- [x] All three panic sites replaced with `MappedErrors` propagation — zero panics reachable from
      valid runtime inputs
- [x] Existing behavior preserved for valid inputs (no regression)
- [x] Each fix covered by at least one unit test for the error path

## Out of Scope

| Feature | Reason |
|---|---|
| Fixing all `unwrap()` across the codebase | Separate effort; most others are in test code |
| Email validation in the DTO layer | Separate concern; this fix targets the send path |
| Hot-reloading Tera templates | Config/ops concern; out of scope here |
| mTLS, rate limiting, or other concerns | Different features |

---

## User Stories

### P1: Notifier email send returns error on invalid address ⭐ MVP

**User Story:** As the gateway, when I call `RemoteMessageWrite::send()` with a malformed
email address, I want to receive a `MappedErrors` error so that the caller can log and
continue rather than the process crashing.

**Why P1:** A single bad email in the database currently takes down the notifier adapter for
all subsequent sends in the same process lifetime.

**Acceptance Criteria:**

1. WHEN `message.from` contains a malformed email string THEN `send()` SHALL return
   `Err(MappedErrors)` with a descriptive message, not panic.
2. WHEN `message.to` contains a malformed email string THEN `send()` SHALL return
   `Err(MappedErrors)` with a descriptive message, not panic.
3. WHEN `lettre::Message::builder().body()` returns an error THEN `send()` SHALL return
   `Err(MappedErrors)`, not panic.
4. WHEN all email fields are valid THEN `send()` SHALL behave identically to today (no regression).

**Files:** `adapters/notifier/src/repositories/remote_message_sending.rs`

**Independent Test:** Unit test with a malformed `from` address → assert `Err`, not panic.

---

### P1: Tera startup degradation instead of panic ⭐ MVP

**User Story:** As an operator, when the templates directory is missing or contains a broken
template, I want the process to start and return a `MappedErrors` on template access so that
I get a clear error message rather than a process crash.

**Why P1:** A missing `TEMPLATES_DIR` env var or deleted directory currently crashes the entire
gateway at startup with a non-descriptive `thread 'main' panicked` message.

**Acceptance Criteria:**

1. WHEN the templates directory does not exist or is unreadable THEN the process SHALL NOT
   panic; it SHALL log a warning and Tera callers SHALL receive `Err(MappedErrors)`.
2. WHEN a template file is syntactically invalid THEN the process SHALL NOT panic; callers
   SHALL receive `Err(MappedErrors)` with the template name and parse error.
3. WHEN templates load successfully THEN behavior SHALL be identical to today.

**Files:** `core/src/settings.rs`

**Implementation note:** `lazy_static!` requires a concrete value. Replace the `panic!` branch
with an empty `Tera::default()` (valid, renders nothing) and let render-time calls propagate
errors. Alternatively, wrap in `Result<Tera, String>` using `std::sync::OnceLock` (available
since Rust 1.70, already required by this crate).

**Independent Test:** Point `TEMPLATES_DIR` at a non-existent path → assert no panic, assert
template render returns `Err`.

---

### P1: Service host selection returns error on empty hosts ⭐ MVP

**User Story:** As the router, when a service is configured with an empty `hosts` list, I want
to receive a `MappedErrors` error so that routing fails gracefully with a clear message rather
than panicking.

**Why P1:** Any misconfigured service (hosts: []) takes down the routing hot path.

**Acceptance Criteria:**

1. WHEN `service.hosts` is empty THEN the host-selection logic SHALL return `Err(MappedErrors)`
   with message `"Service has no configured hosts"`, not panic.
2. WHEN `service.hosts` has one or more entries THEN behavior SHALL be identical to today.

**Files:** `core/src/domain/dtos/service.rs:90`

**Independent Test:** Construct a `Service` with `hosts: vec![]` → assert `Err`, not panic.

---

## Edge Cases

- WHEN `from` email is syntactically valid but the domain is unreachable THEN behavior is
  unchanged (SMTP errors are already handled).
- WHEN `TEMPLATES_DIR` is set but empty (zero templates) THEN Tera initializes successfully;
  any render call for a missing template returns `Err` at render time (existing Tera behavior).
- WHEN `hosts` has exactly one entry THEN it is always selected (no change needed).

---

## Requirement Traceability

| Requirement ID | Story | Status |
|---|---|---|
| NPANIC-01 | P1: Notifier — malformed `from` email | Done |
| NPANIC-02 | P1: Notifier — malformed `to` email | Done |
| NPANIC-03 | P1: Notifier — lettre builder error | Done |
| NPANIC-04 | P1: Tera — missing/unreadable templates dir | Done |
| NPANIC-05 | P1: Tera — malformed template file | Done |
| NPANIC-06 | P1: Service — empty hosts list | Done |

---

## Success Criteria

- [x] `cargo test` passes with no regressions
- [x] All six NPANIC-xx requirements have a corresponding test asserting the error path
- [x] `grep -n '\.unwrap()' adapters/notifier/src/repositories/remote_message_sending.rs`
      returns zero results
- [x] `grep -n 'panic!' core/src/settings.rs` returns zero results
- [x] `grep -n '\.unwrap()' core/src/domain/dtos/service.rs` returns zero results (production
      code; test-only unwraps acceptable)
