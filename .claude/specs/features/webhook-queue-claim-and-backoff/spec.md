# Webhook queue: multi-pod claim, exponential backoff, terminal state

**Issue:** `LepistaBioinformatics/mycelium#192`
**Status:** Implemented — gates green (486 tests, 0 failures; 482 offline plus 4 that skip unless
`MYC_TEST_DATABASE_URL` points at a Postgres, all 4 run and passed against a throwaway
`postgres:16`), awaiting user UAT before commit
**Scope:** Large (core DTO + config + both diesel adapters + use case + dispatcher + pg migration)
**Branch:** `feat/webhook-queue-claim-and-backoff` (from `develop` @ `6c41e83a`)
**Related:** constraint C-9 of `LepistaBioinformatics/mycelium-monorepo#19` (instance federation),
whose outbox is specified against the email queue's semantics rather than these

---

## Problem

Four defects on the webhook delivery path. The email queue in the same codebase already solved the
first one; this is an inconsistency, not an unknown.

### P-1 — no row locking, duplicate delivery across replicas

`adapters/diesel_postgres/src/repositories/webhook/webhook_fetching.rs:235-266` selects pending
events with no `.for_update()` and no `.skip_locked()`. The only defence is the random jitter in
`ports/api/src/dispatchers/webhook_dispatcher.rs:56-64`, whose own comment admits it exists "to
avoid the simultaneous consumption of the same event over multiple containers". Jitter shifts the
odds; it does not make the overlap impossible. Two pods whose ticks land close enough both dispatch
the same event.

`adapters/diesel_postgres/src/repositories/message/local_message_read.rs:88-89` is the in-repo
precedent: `for_update().skip_locked()` inside a transaction that stamps the claim.

### P-2 — no backoff

The current query carries **no predicate on `attempted` at all** — that is the whole reason retry is
a flat `consume_interval_in_secs` (default 30). A downstream that is down or rate-limiting is hit at
a constant rate until `max_attempts` is spent, which with the defaults means the event is abandoned
~2.5 minutes after the first failure.

### P-3 — no terminal state, no alert

`WebHookExecutionStatus` (`core/src/domain/dtos/webhook/responses.rs:13-43`) is
`Pending | Success | Failed | Skipped | Unknown`. Once `attempts >= max_attempts` the row simply
stops matching the selector and sits in `Failed` forever. Nothing distinguishes "failed, will be
retried" from "failed, given up on", and nothing is emitted when an event crosses that line.

### P-4 — unbounded HTTP client

`core/src/use_cases/support/dispatch_webhooks.rs:129-140` builds its `reqwest::Client` with
`danger_accept_invalid_certs(...)` and nothing else — no request timeout, no connect timeout. A
downstream that accepts the connection and never answers holds the dispatcher's future open
indefinitely. This is also what makes any claim-window invariant unprovable today: with no timeout
there is no worst case to bound.

---

## Scope decisions

| # | Decision | Rationale |
|---|---|---|
| D-1 | **Terminal state + structured alert, no new API surface.** | There is no endpoint exposing `webhook_execution` today — the two `WebHookExecutionStatus` references in `update_webhook.rs:204,382` are test mocks, not controllers. A list/requeue endpoint is a feature, not a defect fix. User decision, 2026-09-20. |
| D-2 | **Backoff lands in both adapters; the claim stays Postgres-only.** | The backoff predicate is portable. Row locking is not, and standalone (SQLite) is single-process by construction — the same reason `adapters/diesel_sqlite/.../local_message_read.rs` kept the plain query when the Postgres twin gained the claim. User decision, 2026-09-20. |
| D-3 | **Three separate clocks: `attempted` drives backoff, a new `claimed_at` drives the lease, `attempts` drives the tier and the terminal transition.** | The email queue stamps `attempted` at claim time, which forces retry back-off and crash recovery onto the same knob — a trade-off its own comment flags as unavoidable there. Keeping them apart is what makes real exponential spacing possible from the first retry. The lease clock **must** start when the row is claimed, not when it was last attempted, or a row whose `attempted` is already older than the window goes stale the instant it is claimed and P-1 stays open. That requires a written timestamp; `status = 'processing'` alone cannot carry it. |
| D-4 | **`claimed_at` exists on Postgres only.** | D-2 means SQLite never claims. The two backends have independent `schema.rs` and independent migration sets, so the column simply does not exist there — no SQLite migration, no SQLite model change. |
| D-5 | **`accept_invalid_certificates` keeps defaulting to `true`.** | Flipping it breaks every deployment pointing at an internal self-signed endpoint. Out of scope; noted for a separate decision. |
| D-6 | **The dispatcher's sequential per-event loop is left alone.** | Making it concurrent would bound the batch wall-clock and let the crash-recovery window shrink, but it changes the dispatch concurrency profile (N events × M hooks in flight) and risks pool starvation. The window is sized against the sequential worst case instead. |
| D-8 | **The retry policy travels as a `fetch_execution_event` argument, not as an injected shaku field.** | `main.rs` wires the repositories at **two** sites (full mode and postgres-only, 1062/1094 and 1321/1338 for the email twin). A `#[shaku(default)]` field missed at one of them yields `0`, and a zero window makes the lease predicate always-true — the fix would silently not apply in one mode. The trait already carries `max_events` and `max_attempts`; the dispatcher already holds `config.webhook`. Passing a `WebHookRetryPolicy` alongside them keeps the wiring in one place and makes a missed site a compile error. |
| D-7 | **Every early-return path in `dispatch_webhooks` persists a failed attempt before returning the error.** | Those paths — `decode_payload`, the `list_by_trigger` error arm, the paginated arm, `derive_kek_bytes`, `get_or_provision_dek`, `decrypt_me`, `Client::builder().build()` — return today without ever calling `update_execution_event`. Under a claim they would strand the row in `processing` **with `attempts` unchanged**, so it would never reach the terminal state and would loop forever at lease speed. Bumping `attempts` and writing the status at those exits both releases the claim and lets them terminate, which is what R-3 actually promises. |

---

## Requirements

| ID | Requirement |
|---|---|
| R-1 | Two Postgres replicas ticking simultaneously never dispatch the same execution event. |
| R-2 | The interval between attempt *n* and attempt *n+1* grows exponentially in *n*, bounded by a configured ceiling. |
| R-3 | An event whose `attempts` reaches `max_attempts` reaches a distinct terminal status and is never selected again — including when the attempt failed before any HTTP request was made. |
| R-4 | Crossing into that terminal status emits a structured `tracing::error!` carrying the event id, trigger and attempt count. |
| R-5 | An event claimed by a pod that then dies is reclaimed by another pod a bounded, configured time **after the claim**. |
| R-6 | Every outbound webhook request has a request timeout and a connect timeout, both configurable. |
| R-7 | Standalone (SQLite) gets R-2, R-3, R-4 and R-6. R-1 and R-5 do not apply to a single process. |
| R-8 | Existing deployments keep working without a config change — every new knob has a default. |
| R-9 | An unrecognised status string read from either backend degrades to `Unknown` instead of panicking. |

On R-9, stated precisely, because the obvious version of the argument is wrong. Both fetching
adapters do `.map(|s| WebHookExecutionStatus::from_str(&s).unwrap())`, and `from_str` errored on
anything it did not know — so the tempting claim is that during a rolling deploy an old pod reads
the `processing` a new pod wrote and panics its dispatcher task. **It does not**: the old query
filters `status IN ('pending', 'failed')` in SQL, so a `processing` or `exhausted` row is never
returned to it and that string never reaches `from_str`. The same holds on rollback.

R-9 is still worth landing, and landing first, for what it actually is: an `unwrap` on a
background-task path where the only thing standing between a stored string and a panic is a filter
in a different function. Any future caller passing a wider status filter — an operator endpoint, a
reaper, a migration backfill — inherits the panic. Degrading to `Unknown` costs three lines and the
row is simply not selected, which is recoverable.

---

## Design

### Status machine

Two new variants on `WebHookExecutionStatus`:

```
Pending ──claim──▶ Processing ──dispatch ok───▶ Success
                        │
                        ├──attempt failed, attempts < max──▶ Failed ──backoff elapsed, claim──▶ Processing
                        │
                        ├──attempt failed, attempts >= max─▶ Exhausted   (terminal)
                        │
                        └──pod died────────────────────────▶ (stale, reclaimed after the lease window)

Pending ──no hook registered for the trigger──▶ Skipped
```

"Attempt failed" covers both a 4xx/5xx from a hook and any of the D-7 early returns.
`Exhausted` is terminal: it is never in the selector and nothing transitions out of it.
`Unknown` keeps its current meaning, widened by R-9 to "absent or unrecognised".

### Eligibility predicate

Selection is the OR of two branches, both also requiring `attempts < max_attempts`:

```
(status IN <requested>  AND (attempted IS NULL OR attempted < now - backoff(attempts)))
OR
(status = 'processing'  AND claimed_at < now - visibility_timeout)          -- Postgres only
```

`backoff(n) = min(retry_base_in_secs * 2^n, retry_cap_in_secs)`.

The exponent is computed **in Rust**, not in SQL: the first branch is expanded into one OR-term per
attempt tier `n` in `0..max_attempts`, each comparing against its own precomputed cutoff, every
value bound. This keeps the whole thing inside the diesel DSL and identical on both backends —
`power()` / `interval` arithmetic would be Postgres-only and fork the two adapters. With
`max_attempts` defaulting to 5 the chain is five terms.

The second branch is always included regardless of what the caller asked for: a stale claim must be
recoverable whatever status filter the dispatcher passed.

### The claim (Postgres only)

One transaction, mirroring `local_message_read.rs`:

1. `SELECT ... FOR UPDATE SKIP LOCKED LIMIT max_events`
2. `UPDATE ... SET status = 'processing', claimed_at = now() WHERE id = ANY(claimed_ids)`
3. return the rows

`attempts` and `attempted` are **not** touched by the claim — `dispatch_webhooks` bumps both through
`update_execution_event` when the attempt resolves, including on the D-7 paths.

SQLite runs the first branch alone, without the transaction, the lock hints, the `processing` write
or `claimed_at`: one process, one dispatcher, nothing to race against. A crash there leaves rows in
their prior status, which is already correct.

### Schema (Postgres only)

- New migration `adapters/diesel_postgres/sql/migrations/<date>_01_webhook_execution_claim.sql`:
  `ALTER TABLE webhook_execution ADD COLUMN claimed_at TIMESTAMPTZ DEFAULT NULL;` plus
  `CREATE INDEX IF NOT EXISTS idx_webhook_execution_claim ON webhook_execution (status, attempts, attempted);`
  — the analogue of `20260722_02_message_queue_claim_index.sql`, which exists because the email
  claim's status-filtered ordered scan needed backing.
- `sql/up.sql` gains the column and the index so a fresh install matches a migrated one.
- `adapters/diesel_postgres/src/schema.rs` and `src/models/webhook_execution.rs` gain the field.
  `claimed_at` stays internal to the adapter: it is a lease detail, not domain state, so it is
  **not** added to `WebHookPayloadArtifact`.

### Config (`[webhook]`)

| Key | Type | Default | Meaning |
|---|---|---|---|
| `requestTimeoutInSecs` | `SecretResolver<u64>` | 30 | Whole-request timeout on the reqwest client |
| `connectTimeoutInSecs` | `SecretResolver<u64>` | 10 | Connect phase only |
| `retryBaseInSecs` | `SecretResolver<u64>` | 30 | `backoff(0)` |
| `retryCapInSecs` | `SecretResolver<u64>` | 3600 | Ceiling on `backoff(n)` |
| `visibilityTimeoutInSecs` | `SecretResolver<i64>` | 900 | How long a claimed row stays un-reclaimable |

With the defaults, attempts are spaced 30s, 60s, 120s, 240s, 480s — against today's flat 30s.

**Invariant on `visibilityTimeoutInSecs`:** it must exceed the worst-case wall-clock of one whole
claimed batch, because the batch is marked `processing` up front and dispatched sequentially
(`webhook_dispatcher.rs:127`). The HTTP part of that bound is
`consumeBatchSize * requestTimeoutInSecs` = 25 × 30 = 750s with the defaults — a **floor**, not the
whole cost: each event also pays `list_by_trigger`, `derive_kek_bytes`, `get_or_provision_dek` and a
sequential `decrypt_me` loop over the registered hooks. 900 is that floor plus slack, and an
operator raising the batch or the timeout must raise this proportionally, or a live-but-slow pod has
its un-dispatched rows reclaimed and double-sent. The invariant only became expressible with R-6 —
before the timeouts there was no worst case at all.

### Blast radius of the two new variants

`Display`, `FromStr`, the `ToSchema` derive (so the OpenAPI document), the dispatcher's status
filter at `webhook_dispatcher.rs:84-87`, the status decision at `dispatch_webhooks.rs:247-252`, both
fetching adapters, and the SQLite registration test at `webhook_registration.rs:242-261`. The
webapp does not reference the enum and the SDK contract in CLAUDE.md is `Profile`-only, so neither
needs parity work.

---

## Verification

| Requirement | How it is proven |
|---|---|
| R-2 | `retry_policy.rs`: tier growth, the cap, saturation at `u32::MAX`, and a cap below the base. Plus the boundary itself, on both backends: a failure 45s old is held inside its 60s tier and one 75s old is released. The SQLite version doubles as the only check that TEXT timestamps compare chronologically. |
| R-3 | `dispatch_webhooks.rs`: the last allowed failure becomes `Exhausted`, one short of it stays `Failed`, `Success`/`Skipped` are never escalated, a saturated `u8` counter still terminates, and an aborted attempt — one that never reached an HTTP request — is persisted before the error is returned. |
| R-4 | The status assertions above; the log line itself is inspected during UAT. |
| R-6 | `webhook_config_defaults_when_fields_absent`, extended. A second test asserts the `visibilityTimeoutInSecs` invariant against the other two defaults, so changing one default without the others fails the build. |
| R-9 | `responses.rs`: every variant round-trips through `Display`/`FromStr`, and four unrecognised strings degrade to `Unknown`. |
| **R-1, R-5** | **Not reachable offline.** Four tests in `webhook_fetching.rs` behind `MYC_TEST_DATABASE_URL`: a claimed batch is invisible to the next claim; two simultaneous claims never overlap and never lose an event; a `processing` row inside the window is not stolen and one past it is reclaimed; the back-off boundary holds. Readiness is the **second** "database system is ready to accept connections" in `docker logs` — `pg_isready` succeeds against initdb's temporary server. |

The concurrency test's observed split is 40/0, not 20/20. That is `SKIP LOCKED` behaving correctly —
the first transaction locks every candidate and the second skips all of them rather than waiting —
and the property under test still holds: each event is claimed exactly once. The negative control is
clear, since the old code would have returned all 40 to both threads and failed both assertions.

Gates before any commit, from `modules/mycelium-api-gateway/`:
`cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all`.

---

## Notes for UAT

- `settings/config.dev.for-docker.toml` matches `.gitignore:15` (`config.dev*.toml`). It was updated
  on disk, so a dev stack started from this checkout already carries the new keys — but they will not
  appear in the diff and a fresh clone will not have them. The defaults make that harmless; it only
  matters if the values are tuned during UAT and the tuning is expected to survive.
- The log line to look for is `Webhook delivery abandoned after the last allowed attempt`, at error
  level, carrying `webhook_execution_id`, `trigger` and `attempts`.

## Recorded concerns, not addressed here

- `WebhookConfig` now carries nine public fields, against the five the object-calisthenics rule
  allows. Splitting it into sub-tables (`[core.webhook.retry]`, …) would satisfy the rule but change
  the shape of a published config surface, which R-8 rules out for this change.
- `dispatch_webhooks` remains well over the 20-line function limit. The three helpers extracted here
  (`resolve_hook_secrets`, `build_dispatch_client`, `record_attempt`) pulled it down but did not
  close the gap; the rest is pre-existing and was left alone.

## Out of scope

- Any REST/RPC surface over `webhook_execution` — see D-1.
- Flipping `accept_invalid_certificates` — see D-5.
- Concurrent dispatch of a claimed batch — see D-6.
- A dead-letter *table*. The terminal status plus the `attempts`/`propagations` already on the row
  carry the same information; a second table would need its own retention and its own operator
  surface, which D-1 rules out.
