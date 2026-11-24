# Tasks — webhook queue claim, backoff and terminal state

Origin: [spec.md](spec.md). Gates for every task, from `modules/mycelium-api-gateway/`:
`cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all`.
No commit — not even in the submodule — until the user has tested and approved.

## Wave 0 — rolling-deploy safety (ships before anything writes a new status)

- [x] **T1** (R-9) — `WebHookExecutionStatus::from_str` returns `Unknown` for an unrecognised
  string instead of `dto_err`. Both fetching adapters `.unwrap()` that result, on a background
  task, guarded only by a status filter written in another function.
  **Where:** `core/src/domain/dtos/webhook/responses.rs:57-70`.
  **Done when:** a unit test covers an unknown string; the round-trip test covers the known ones.
  **Not** the reason: the rolling-deploy panic does not exist — the old query filters
  `status IN ('pending', 'failed')` in SQL, so the new strings never reach `from_str` on an old
  pod. See the R-9 note in spec.md.

## Wave 1 — core vocabulary and config (independent of each other)

- [x] **T2** (R-3) — Add `Processing` and `Exhausted` to `WebHookExecutionStatus`, with `Display`
  (`"processing"` / `"exhausted"`), `FromStr` and doc comments in the style of the existing
  variants.
  **Where:** `core/src/domain/dtos/webhook/responses.rs:13-70`.
  **Depends on:** T1.
  **Watch:** the `ToSchema` derive puts both in the OpenAPI document; the SQLite registration test
  at `adapters/diesel_sqlite/.../webhook_registration.rs:242-261` asserts on statuses.

- [x] **T3** (R-6, R-2) — Add `requestTimeoutInSecs` (30), `connectTimeoutInSecs` (10),
  `retryBaseInSecs` (30), `retryCapInSecs` (3600) and `visibilityTimeoutInSecs` (900) to
  `WebhookConfig`, each with its `default_*` fn.
  **Where:** `core/src/models/webhook_config.rs`.
  **Done when:** `webhook_config_defaults_when_fields_absent` is extended (not duplicated) and
  passes.

- [x] **T4** (D-8) — `WebHookRetryPolicy { retry_base_in_secs, retry_cap_in_secs,
  visibility_timeout_in_secs }` plus its `backoff(attempt) -> Duration` method
  (`min(base * 2^n, cap)`, saturating).
  **Where:** `core/src/domain/dtos/webhook/` (new module, re-exported from `mod.rs`).
  **Done when:** unit tests cover tier growth, the cap, and that a large `n` saturates rather than
  overflowing.

## Wave 2 — the use case

- [x] **T5** (R-6) — Give the reqwest client `.timeout()` and `.connect_timeout()` from T3.
  **Where:** `core/src/use_cases/support/dispatch_webhooks.rs:129-140`.
  **Depends on:** T3.

- [x] **T6** (R-3, R-4, D-7) — Terminal transition and alert:
  - after an attempt resolves, `Exhausted` instead of `Failed` when
    `attempts >= max_attempts`, with `tracing::error!` carrying id, trigger and attempts;
  - every early return that today leaves without calling `update_execution_event` —
    `decode_payload`, the `list_by_trigger` error arm, the paginated `_` arm, `derive_kek_bytes`,
    `get_or_provision_dek`, the `decrypt_me` loop, `Client::builder().build()` — first bumps
    `attempts`, writes `Failed`/`Exhausted`, persists, and only then returns the error.
  **Where:** `core/src/use_cases/support/dispatch_webhooks.rs`.
  **Depends on:** T2.
  **Note:** neither adapter's `update_execution_event` writes the `payload` column, so persisting a
  failure after `decode_payload` has failed is safe with the artifact as received.

## Wave 3 — adapters

- [x] **T7** (R-5) — Postgres schema: migration
  `sql/migrations/<date>_01_webhook_execution_claim.sql` adding `claimed_at TIMESTAMPTZ DEFAULT
  NULL` and `idx_webhook_execution_claim ON webhook_execution (status, attempts, attempted)`;
  same in `sql/up.sql`; field added to `src/schema.rs` and `src/models/webhook_execution.rs`.
  **Where:** `adapters/diesel_postgres/`.
  **Watch:** `claimed_at` stays inside the adapter — it is not added to `WebHookPayloadArtifact`.

- [x] **T8** (R-1, R-2, R-5) — Postgres `fetch_execution_event` becomes a claim transaction:
  boxed query, the tier OR-chain plus the stale-`processing` branch, `for_update().skip_locked()`,
  then `UPDATE ... SET status = 'processing', claimed_at = now()` on the claimed ids.
  **Where:** `adapters/diesel_postgres/src/repositories/webhook/webhook_fetching.rs:232-292`.
  **Depends on:** T4, T7.
  **Reuses:** `adapters/diesel_postgres/src/repositories/message/local_message_read.rs:74-110` —
  same transaction shape, same `SAFETY INVARIANT` comment style.

- [x] **T9** (R-2, R-7) — SQLite `fetch_execution_event` gains the tier OR-chain only: no
  transaction, no lock hints, no `processing` write, no `claimed_at`.
  **Where:** `adapters/diesel_sqlite/src/repositories/webhook/webhook_fetching.rs:171-237`.
  **Depends on:** T4.
  **Watch:** `attempted` is TEXT there; `naive_timestamp_to_text` is fixed-width in its date part so
  lexicographic comparison is chronological — bind the formatted cutoff, do not build a string.

- [x] **T10** (D-8) — Extend the `WebHookFetching::fetch_execution_event` signature with the retry
  policy and update every implementation and mock.
  **Where:** `core/src/domain/entities/webhook/webhook_fetching.rs:41`, both adapters, and the two
  test mocks at `core/src/use_cases/role_scoped/system_manager/webhook/update_webhook.rs:199,377`.
  **Depends on:** T4.

## Wave 4 — wiring and docs

- [x] **T11** — Dispatcher passes the policy built from `config.webhook`.
  **Where:** `ports/api/src/dispatchers/webhook_dispatcher.rs:71-90`.
  **Depends on:** T10.

- [x] **T12** (R-8) — Document the five new keys in the `[core.webhook]` block of
  `settings/config.full.example.toml`, `config.postgres-only.example.toml`,
  `config.standalone.example.toml` and `config.dev.for-docker.toml`, commented out at their
  defaults, matching the surrounding style. Include the `visibilityTimeoutInSecs` invariant as a
  comment.
  **Note:** `config.dev.for-docker.toml` matches `.gitignore:15` (`config.dev*.toml`), so it was
  updated on disk but will not appear in the diff.

## Wave 5 — verification

- [x] **T13** (R-1, R-5) — Concurrency proof against a throwaway Postgres: two concurrent claims
  return disjoint id sets; a row left `processing` past the window is reclaimed and one inside it is
  not. Readiness by grepping `docker logs` for the **second** "database system is ready to accept
  connections" — `pg_isready` succeeds against initdb's temporary server.
  **Depends on:** T8.
  **Note:** this is a manual/scripted check, not a workspace test — it needs a live Postgres.

## Extra, not planned

- [x] **T14** — `propagations` decoding no longer panics on a stored JSON `null`, and `None` is
  written as a SQL NULL rather than as a JSON `null`, in **both** adapters.
  **Where:** `adapters/diesel_{postgres,sqlite}/src/repositories/webhook/webhook_{fetching,updating}.rs`.
  **Why it appeared:** the new back-off test in T9 tripped it —
  `serde_json::from_value::<Vec<HookResponse>>(Value::Null)` is `invalid type: null, expected a
  sequence`, and both adapters `.unwrap()` it inside the dispatcher task. Latent before this
  feature; T6 makes it routine, since every early-return path now persists an attempt with no
  propagations to show for it.

## Parallelisation

- T2, T3, T4 are independent once T1 lands.
- T5 needs only T3; T6 needs only T2 — they touch the same file, so run them in sequence.
- T7 is independent of the core work; T8 needs T4 + T7; T9 needs T4.
- T10 fans out to everything that implements the trait, so it lands with T8/T9 in one build.
