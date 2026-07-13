# Design: Staff Bootstrap (Autonomous First-Login Onboarding)

**Spec:** `.claude/specs/features/staff-bootstrap/spec.md`
**Status:** Ready for tasks

---

## 1. Resolving the spec's open concerns

| ID | Resolution |
|---|---|
| OC-1 | Accepted as documented edge case (see spec §5/§6) — no active cleanup. An orphaned plain `User` from a lost CAS race is indistinguishable from an ordinary abandoned magic-link signup and requires no special handling. |
| OC-2 | **D-1 (revised — the original "single transaction" idea was not implementable):** there is no cross-repo transaction anywhere in this codebase — verified against the closest precedent, `create_seed_staff_account`, which calls `user_registration_repo.get_or_create(user)` and `account_registration_repo.get_or_create_user_account(...)` as two independent, non-transactional calls (each port impl grabs its own pooled connection). `claim_staff_bootstrap` follows the same shape: **call `InstanceSettingsRegistration::get_or_create(STAFF_BOOTSTRAP_KEY, ...)` first** (a single atomic `INSERT ... ON CONFLICT (key) DO NOTHING` — atomic on its own, no transaction needed); only if it returns `Created` (this call won the key) does the use-case proceed to create/upgrade the `Account` as `AccountType::Staff`. If it returns `NotCreated` (lost the race — key already existed), the use-case returns "already initialized" immediately and **never touches the Account repo at all** — there is nothing to roll back. Residual risk: the insert succeeds but the subsequent Account-upgrade call fails (DB blip) — the instance is left permanently "claimed" with no Staff account. Accepted, same class as OC-1: recoverable via the manual CLI path (SB-R9), since `create_seed_staff_account` doesn't check `instance_settings` at all. |
| OC-3 | Confirmed: the port layer (`ports/api`) builds the full bootstrap URL from `life_cycle_settings.domain_url` for the boot-time log line — the same pattern `request_magic_link_url` already uses (`ports/api/src/rest/role_scoped/beginners/user_endpoints.rs:987-1006`). The core use-case never constructs a path. |
| OC-4 | Constant-time secret comparison: `subtle::ConstantTimeEq` — **not** `ring::constant_time::verify_slices_are_equal`, which is deprecated in the pinned `ring` 0.17 (`#[deprecated(note = "... not intended for external use with no promises regarding side channels")]` — using it would be worse than no constant-time guarantee at all, since it advertises safety it no longer promises). `subtle` is already a **direct** dependency of `lib/http_tools`, used for exactly this purpose (`verify_webhook_secret.rs`: length check, then `ct_eq(...).unwrap_u8() == 1`) and is already in the workspace's `Cargo.lock` (transitively via `digest`/`orion`/`pasetors`/`rustls`) with a `[workspace.dependencies]` entry — `core/Cargo.toml` only needs `subtle.workspace = true` added, mirroring `http_tools`'s usage, not a new dependency. |

---

## 2. Domain model

**Revised 2026-07-13** — the original design shaped `instance_settings` entirely around the
bootstrap claim (fixed `staff_bootstrap_status`/`claimed_by_user_id`/`claimed_at` columns, `meta
JSONB` as a side reservation). Per explicit user feedback, this inverts: `instance_settings`
becomes a **generalized key/value settings store**; the bootstrap claim is just its first
consumer. The generic DTO/ports live in `core/src/domain/{dtos,entities}/instance_settings.rs` and
know nothing about staff bootstrap specifically — the staff-bootstrap-specific shape is a small
payload struct that the *use-case layer* (not the DTO, not the persistence layer) serializes into
the generic `value` column.

### `core/src/domain/dtos/instance_settings.rs`

**Revised again (2026-07-13, same day, user request):** added `created_by`/`updated_by:
Option<WrittenBy>` so every entry records who wrote it — "seja informativo o suficiente." This
made the bespoke `StaffBootstrapClaim` payload redundant: who-claimed-it and when are already
`created_by`/`created` on the row itself, so the claim's `value` is now just `{}`.

```rust
use crate::domain::dtos::written_by::WrittenBy;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Key of the staff-bootstrap claim entry. Row *presence* under this key is
/// itself the "already claimed" signal -- absence means still pending. No
/// `status` field is stored anywhere; nothing can drift out of sync with row
/// existence.
pub const STAFF_BOOTSTRAP_KEY: &str = "staff_bootstrap";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSetting {
    pub key: String,
    pub value: serde_json::Value,
    pub created_by: Option<WrittenBy>,
    pub updated_by: Option<WrittenBy>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}
```

No secret/token material lives on this DTO at all — the bootstrap secret lives in config
(`AccountLifeCycle`, resolved via `SecretResolver`), never persisted to the DB (see §4).

### `core/src/domain/dtos/written_by.rs` (generalized, SB-R13)

Every existing call site only ever had a `Uuid` (and sometimes not even that, until now).
`WrittenBy` is generalized so `id`/`from` are optional and an independent, optional
base64-encoded `email` carries identity when no User/Account exists yet -- the staff bootstrap
flow's exact situation before any account is created:

```rust
pub struct WrittenBy {
    pub id: Option<Uuid>,          // None when no User/Account exists yet
    pub from: Option<IDSource>,    // None whenever id is None
    pub email: Option<String>,     // base64-encoded, independent of id/from
}
```

Constructors: `new_from_user`/`new_from_account` (unchanged signatures -- zero breakage for the
~10 existing call sites), plus `new_from_user_with_email`/`new_from_account_with_email` (id +
email, used by `claim_staff_bootstrap`, `create_seed_staff_account`,
`create_account_from_existing_user` -- all three already have the email in scope), and
`new_from_email` (email only, no id -- the general case this feature motivates, not currently hit
by any call site but now representable). `impl Display for WrittenBy` renders a compact marker for
logs: `user:<uuid>`, `account:<uuid>;email:<base64>`, or `email:<base64>` alone.

**Backward compatibility:** `id`/`from`/`email` are all `#[serde(default)]`, so pre-existing
`created_by`/`updated_by` JSON on `account`/`webhook` rows (written before this change, shaped
`{"id": "...", "from": "user"}`, no `email` key at all) still deserializes correctly -- no
retroactive data migration, per explicit user instruction ("mantenha a compatibilidade com os
registros anteriores pois é inviável editar todos os campos retroativamente").

**DI wiring location (corrects an earlier draft):** shaku wiring for `SqlAppModule` is **not** a
`ports/api` concern — it lives entirely inside each adapter crate's own
`repositories/mod.rs` `module! { pub SqlAppModule { components = [...] } }` macro
(`adapters/diesel_postgres/src/repositories/mod.rs` and the equivalent
`adapters/diesel_sqlite/src/repositories/mod.rs`, both already list ~44 components this way).
`ports/api` only re-exports whichever `SqlAppModule` type is active via a `#[cfg(feature = ...)]`
switch in `ports/api/src/models/active_backend_modules.rs` — it never lists components itself.
Adding the two new `InstanceSettings*SqlDbRepository` structs to both `module!` blocks is the
entire DI-wiring step.

### Ports (`core/src/domain/entities/`)

**Revised: two traits, not three.** Once row presence *is* the claimed state, there is no separate
CAS-update step to model — claiming a key and creating it are the same operation. The `Updating`
trait from the earlier draft is dropped entirely.

```rust
// instance_settings_fetching.rs
#[async_trait]
pub trait InstanceSettingsFetching: Interface + Send + Sync {
    async fn get(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors>;
}

// instance_settings_registration.rs
#[async_trait]
pub trait InstanceSettingsRegistration: Interface + Send + Sync {
    /// Atomic claim: `INSERT ... ON CONFLICT (key) DO NOTHING`. `Created`
    /// when this call inserted the row -- i.e. this call won the key;
    /// `NotCreated` when the key already existed (another
    /// replica/caller got there first). Either way the returned
    /// `InstanceSetting` reflects the row now in the DB. `created_by`
    /// (SB-R13) is persisted only when this call actually wins.
    async fn get_or_create(
        &self,
        key: String,
        value: serde_json::Value,
        created_by: Option<WrittenBy>,
    ) -> Result<GetOrCreateResponseKind<InstanceSetting>, MappedErrors>;
}
```

*(Implementation note: uses the codebase's existing `GetOrCreateResponseKind`/`FetchResponseKind`
enums from `mycelium_base::entities` — the same pattern `UserRegistration::get_or_create` already
follows.)*

---

## 3. Schema

Additive migration, same pattern as the existing `20260421_01_envelope_encryption.sql`
(`adapters/diesel_postgres/sql/migrations/`) — not folded into the base `up.sql`. **Revised
2026-07-13:** rewritten from a fixed-column singleton to a generic key/value table (edited in
place — this feature hasn't shipped or been applied anywhere yet, so this is a correction, not
migration churn).

**Migration application (Postgres) is manual, matching existing convention.** Verified: neither
`encrypted_dek`/`kek_version` (the existing `20260421_01_envelope_encryption.sql` migration) nor
any other dated migration file appears in `up.sql`, and no script/Dockerfile in the repo applies
`sql/migrations/*.sql` automatically. This project's Postgres schema evolution is operator-applied
(`psql` by hand) — not a gap introduced by this feature. Consequence for boot (§8): if an operator
upgrades the gateway binary without having applied this migration yet, `instance_settings` won't
exist; boot must **not** panic over that (see §8 non-fatal handling) — unlike the hard-fail-at-boot
posture for missing Postgres/Redis/SMTP, a missing *new, optional* table is not core functionality.
SQLite is unaffected (`embed_migrations!` auto-applies on every boot, per SM-T5's existing pattern).

**Postgres** — `adapters/diesel_postgres/sql/migrations/20260713_01_instance_settings.sql`.
**Revised same day (SB-R13):** added `created_by`/`updated_by JSONB`, mirroring the exact
`account`/`webhook` convention (`DEFAULT '{}'::JSONB` sentinel, parsed to `None` by the shared
`parse_optional_written_by` helper):

```sql
CREATE TABLE instance_settings (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    created_by JSONB DEFAULT '{}'::JSONB,
    updated_by JSONB DEFAULT '{}'::JSONB,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);
```

**SQLite** — `adapters/diesel_sqlite/migrations/2026-07-13-000000_instance_settings/up.sql`,
following the project's established Postgres→SQLite type mapping (`Jsonb`→`TEXT`). Unlike Postgres,
`created` has **no DB-side default** — it's supplied explicitly by the repo on insert
(`naive_timestamp_to_text(&Local::now().naive_utc())`), matching every other SQLite table in this
codebase; a `datetime('now')` DB-side default produces SQLite's own text format, which this
project's hand-rolled timestamp parser doesn't accept (discovered via a failing round-trip test
during implementation). `created_by`/`updated_by` are plain nullable `TEXT` (no default), matching
`account`/`webhook`'s SQLite columns exactly:

```sql
CREATE TABLE instance_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_by TEXT,
    updated_by TEXT,
    created TEXT NOT NULL,
    updated TEXT
);
```

Claiming a key is a single atomic insert attempt — no separate CAS-update step:

```sql
-- claim: whoever's insert lands wins the key
INSERT INTO instance_settings (key, value, created_by, created)
VALUES ('staff_bootstrap', $1, $2, $3)
ON CONFLICT (key) DO NOTHING;
-- caller checks rows-affected: 1 = won the claim, 0 = already claimed by someone else
```

---

## 4. Config: the bootstrap secret

Added to `core/src/models/account_life_cycle_config.rs` (already the home for
`domain_url`/`token_expiration`/etc., and already imports `SecretResolver` directly):

```rust
/// Operator-supplied secret gating the one-time staff bootstrap web flow
/// (`/_adm/instance/bootstrap*`). Absent by default — bootstrap stays fully
/// disabled (404) until the operator opts in by setting this.
pub staff_bootstrap_secret: Option<SecretResolver<String>>,
```

Resolved once per request via `SecretResolver::async_get_or_error()`, same as `token_secret`. No
DB storage, no hashing, no generation — this is deliberately the simplest possible secret in the
codebase: a plain configured value, compared with `ring::constant_time::verify_slices_are_equal`
(§1 OC-4) against whatever the operator submits.

---

## 5. Use-cases (`core/src/use_cases/super_users/staff/bootstrap/`)

One file per verb (screaming-architecture Rule 3 — pipeline steps are modules, `mod.rs` is an
orchestrator only):

```
bootstrap/
  mod.rs
  staff_bootstrap_is_pending.rs
  validate_bootstrap_secret.rs
  claim_staff_bootstrap.rs
```

**Revised 2026-07-13:** `ensure_instance_settings_row` is renamed and rewritten as
`staff_bootstrap_is_pending` — fetch-only, since the generalized table has nothing to pre-create at
boot (row presence *is* the state, per DEC-6 revision).

- **`staff_bootstrap_is_pending(instance_settings_fetching_repo)`** — called once at gateway boot
  (from `ports/api/src/main.rs`, after DB pool init, before the HTTP server starts accepting). Calls
  `InstanceSettingsFetching::get(STAFF_BOOTSTRAP_KEY)`; returns `Ok(true)` on `NotFound` (still
  pending), `Ok(false)` on `Found` (already claimed). No crypto, no per-replica branching, no insert
  at all — every replica just observes the same key (DEC-5/DEC-6).
- **`validate_bootstrap_secret(presented, configured, repo)`** — compares `presented` against the
  resolved `staff_bootstrap_secret` using `subtle::ConstantTimeEq` (length check first, then
  `ct_eq(...).unwrap_u8() == 1`, per §1 OC-4 — returns a "not found"/404-mapped error if the config
  field is absent at all — spec SB-R4), then fetches `STAFF_BOOTSTRAP_KEY`, returning `Ok(())` only
  on `NotFound` (still pending); `Found` maps to a `MappedErrors` with `.with_exp_true()`
  (expected/handled, per `rust-error-handling.md`). Called by all three REST handlers before doing
  anything else.
- **`claim_staff_bootstrap(user, account_registration_repo, instance_settings_registration_repo)`**
  — **insert-first, no transaction (D-1 revised)**: builds `WrittenBy::new_from_user_with_email(
  user_id, &user.email.email())` and calls `InstanceSettingsRegistration::get_or_create(
  STAFF_BOOTSTRAP_KEY, json!({}), Some(written_by))` *first* (SB-R13 — `value` carries nothing but
  an empty marker now that `created_by`/`created` on the row already record who claimed it and
  when). Only on `Created` (this call won the key) does it proceed to upgrade/create the caller's
  `Account` as `AccountType::Staff` — itself also stamped with the same `WrittenBy` (id + email).
  On `NotCreated` (lost the race — key already existed), returns an "already initialized"
  `MappedErrors` immediately and **never calls the Account repo** — nothing to roll back, no
  transaction required.

---

## 6. REST layout (`ports/api`)

New top-level public module — neither `rest/staff/` (authenticated) nor
`rest/role_scoped/beginners/` (a different domain concept) fits, per screaming-architecture Rule 2
(names must scream intent):

```
ports/api/src/rest/instance/
  mod.rs                       — configure() registers the 3 routes, nothing else
  bootstrap_claim_page.rs      — GET  /_adm/instance/bootstrap
  bootstrap_request_code.rs    — POST /_adm/instance/bootstrap/request-code
  bootstrap_complete.rs        — POST /_adm/instance/bootstrap/complete
```

| Method | Path | Auth | Behavior |
|---|---|---|---|
| GET | `/_adm/instance/bootstrap` | none | If `staff_bootstrap_secret` unset **or** the `staff_bootstrap` key already exists → 404. Else renders `web/instance-bootstrap-claim` (secret + email form, nothing pre-filled — DEC-4). |

**Accepted enumeration note:** unlike the deliberately-indistinguishable error page (§7), this GET
does leak one bit to any anonymous visitor — "this instance has bootstrap enabled and unclaimed"
(200 vs 404). Nothing actionable follows from that bit alone (the secret still gates every
mutating call), so this is accepted rather than masked with a constant-200 response — masking it
would mean the claim form itself becomes unreachable without already knowing the secret, which
defeats the form's purpose.
| POST | `/_adm/instance/bootstrap/request-code` `{secret, email}` | none | `validate_bootstrap_secret` → calls **unmodified** `request_magic_link` → generic `{sent:true}` |
| POST | `/_adm/instance/bootstrap/complete` `{secret, email, code}` | none | `validate_bootstrap_secret` → calls **unmodified** `verify_magic_link` → on success, `claim_staff_bootstrap` → returns the same `MyceliumLoginResponse` (JWT) `verify_magic_link` would, so the operator is immediately logged in as the new Staff user |

All three annotated `security(())` (public), matching the existing magic-link endpoints.

**Actual operator path (four hops, not three)** — corrected from an earlier draft that
(incorrectly) claimed the browser never leaves `instance/bootstrap/*`. Because `request_magic_link`
is reused **unmodified** (DEC-7), its email still points at the standard
`GET /_adm/beginners/users/magic-link/display` page — the operator's browser genuinely does bounce
out to that URL to read the 6-digit code, then comes back:

1. `GET /_adm/instance/bootstrap` — bootstrap form (secret + email).
2. `POST /_adm/instance/bootstrap/request-code` — internally calls unmodified `request_magic_link`;
   an email is sent containing a link to the **standard, unmodified**
   `/_adm/beginners/users/magic-link/display` page.
3. Operator opens that email and follows the link — this *is* a `magic-link/*` page load, shown as
   plain HTML with the 6-digit code (no bootstrap awareness on this page at all; it doesn't know or
   care that a bootstrap is in progress).
4. Operator returns to the bootstrap tab/page and submits
   `POST /_adm/instance/bootstrap/complete { secret, email, code }`, which internally calls
   unmodified `verify_magic_link` and then `claim_staff_bootstrap`.

Keeping `request_magic_link`/`verify_magic_link` and the display page byte-identical (SB-R10) is
what forces this bounce — the alternative (a bootstrap-only code display, no bounce) would require
touching the email template or the display handler, which contradicts DEC-7. This bounce is
accepted as the cost of zero core changes.

---

## 7. Templates (`templates/web/`)

Two new templates, following the existing `magic-link-display.html` / `magic-link-display-error.html`
visual pattern (same inline-CSS card style, same variable-injection approach):

- `instance-bootstrap-claim.html` — variables: `{{ domain_name }}`. A secret input + an email
  input + submit, posting to `request-code` then `complete` via a small inline script (no new
  frontend framework — mirrors the plain-HTML approach already used). No hidden fields carrying
  server-issued state — everything the operator submits, they typed themselves.
- `instance-bootstrap-error.html` — reused structure of `magic-link-display-error.html`; rendered
  identically (indistinguishable) whether the cause is "bootstrap disabled", "wrong secret", or
  "already claimed" — do not leak which one, same enumeration-avoidance posture as the magic-link
  flow's generic `{sent:true}`.

---

## 8. Boot sequence integration (`ports/api/src/main.rs`)

After DB pool + DI module initialization (where `SqlAppModule`/`SqliteAppModule` already resolve
repository trait objects), before `HttpServer::run()`:

```rust
// Non-fatal: a missing `instance_settings` table (operator upgraded the
// binary without applying the migration yet — see §3) must not panic boot.
// Postgres/Redis/SMTP hard-fail today because they're core functionality;
// this table backs an optional, opt-in feature and must degrade instead.
match staff_bootstrap_is_pending(&instance_settings_fetching_repo).await {
    Ok(true) if life_cycle_settings.staff_bootstrap_secret.is_some() => {
        let claim_url = format!("{domain_url}/_adm/instance/bootstrap");
        tracing::info!(claim_url, "staff bootstrap pending — visit this URL with your configured bootstrap secret to claim the initial staff account");
    }
    Ok(_) => {
        // Already claimed, or secret unset → no log line at all (SB-R4)
    }
    Err(err) => {
        tracing::error!(%err, "staff bootstrap unavailable this boot (instance_settings table missing or unreachable) — gateway continues without it");
    }
}
```

Far simpler than the rejected generate-and-log design: no per-replica winner/loser branch, no
token to keep out of the log, and now (post-revision) no boot-time write at all — just a fetch.

---

## 9. Consistency with the CLI path (SB-R9)

`create_seed_staff_account` (unchanged per DEC-1) gets one additive call appended at its call site
in `ports/cli/src/cmds/accounts.rs` (not inside the core use-case itself, to keep DEC-1's "untouched"
promise literal at the use-case level): after a successful seed-account creation, best-effort call
`InstanceSettingsRegistration::get_or_create(STAFF_BOOTSTRAP_KEY, claim_value)` and log (not fail)
on error, per spec SB-R9. Same call the web flow's `claim_staff_bootstrap` uses — whichever path
gets there first wins the key.

---

## 10. Dependency/crate impact

One line added to `core/Cargo.toml`: `subtle.workspace = true` (already a workspace-level
dependency, already used directly by `lib/http_tools` for the identical constant-time-comparison
purpose — §1 OC-4). No hashing, no random generation needed at all now that the secret lives in
config. No changes to `lib/config`, `adapters/kv_db`, `adapters/moka_cache`, or `adapters/notifier`.

---

## 11. Non-regression verification plan

- Diff `core/src/use_cases/role_scoped/beginner/user/{request_magic_link,verify_magic_link}.rs`
  against `develop` post-implementation — MUST be empty (SB-R10).
- `cargo test -p myc-core magic_link` — existing tests MUST still pass unchanged.
- Full gate: `cargo fmt --all -- --check && cargo build --workspace && cargo test --workspace --all`.
- Standalone gate: `cargo build --no-default-features --features standalone` + its test suite, with
  the new SQLite migration present.
- Regression check specific to this design: a deploy with `staff_bootstrap_secret` unset must show
  zero behavioral change from before this feature existed (all three new routes 404, no new log
  lines, `instance_settings` row still created silently in the background for future opt-in).
