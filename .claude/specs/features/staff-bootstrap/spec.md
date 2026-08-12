# Feature Spec: Staff Bootstrap (Autonomous First-Login Onboarding)

**Feature:** staff-bootstrap
**Milestone:** M3 — Auth Evolution
**Status:** Specified
**Created:** 2026-07-13
**Scope:** Complex (new instance-level domain concept, dual-adapter persistence, a new
unauthenticated attack surface that must be provably single-use, reuses existing magic-link infra)

---

## 1. Objective

Today the only way to create the system's first Staff account is `mycelium-cli accounts
create-seed-account`, which requires typing the raw database URL and a password into the CLI at
execution time (`ports/cli/src/functions/try_to_resolve_database_url.rs`). This is incompatible
with a fully autonomous deploy (e.g. dokploy): nobody wants to shell into a running container to
hand it DB credentials, and the credential-typing step contradicts the gateway's own
`SecretResolver`/Vault config model that every other component uses.

Replace the credential-driven CLI path (for autonomous deploys) with a **self-service, one-time,
web-based onboarding flow served by the running gateway itself**: once the gateway is up (DB
already connected via its normal config), the operator opens a bootstrap URL, proves they are the
deployer (via a secret they configured at deploy time, not the DB itself), types the initial staff
email themselves, and authenticates via the existing magic-link mechanism. No external interface
(API caller, admin UI, DB client) is required — only a browser and the deploy-time config the
operator already holds.

This must remain safe under **multiple replicas sharing one database** (the normal deploy
topology — see `project_production_topology` memory) — the flow cannot rely on in-process state or
"check-then-create" races. It must also degrade correctly to the **single-instance standalone
build**, where there is exactly one replica.

### Target use case

- `dokploy/mycelium-api-gateway` (and any similarly autonomous container deploy): gateway boots
  fully wired (Postgres + SMTP already configured via `config.toml`/Vault, per repo convention),
  no human has DB access, staff account is created once by whoever opens the bootstrap link.

---

## 2. Verified current state (baseline)

Established by direct code inspection on 2026-07-13.

| Concern | Reality |
|---|---|
| CLI init command | `mycelium-cli accounts create-seed-account <email> <account_name> <first_name> <last_name>` (`ports/cli/src/cmds/accounts.rs:26-59`). Prompts password interactively (`rpassword`). Resolves `DATABASE_URL` from env var or an interactive prompt (`ports/cli/src/functions/try_to_resolve_database_url.rs:6-26`) — **not** via `SecretResolver`/Vault. Builds a one-off `SqlAppModule` directly from the raw URL, bypassing the config/Vault-backed DI the API server uses. |
| Core use-case | `create_seed_staff_account` (`core/src/use_cases/super_users/staff/account/create_seed_staff_account.rs:29-103`) — a real core use-case, but explicitly out-of-band ("could not be exposed through API ports", doc comment lines 23-27). Errors if the user already exists. |
| Magic link (existing, reusable as-is) | `request_magic_link` / `verify_magic_link` (`core/src/use_cases/role_scoped/beginner/user/`). Two-phase token: UUID `token` (consumed on display) + 6-digit `code` (consumed on verify), stored as JSONB in the `token` table (Postgres) / mirrored in SQLite. Delivery via `dispatch_notification` + SMTP (`SecretResolver`-backed) or, in standalone builds, Stub/File transport (SM-R7). `verify_magic_link` auto-creates a minimal `User` if none exists — **no pre-existing account required**. Endpoints: `POST /_adm/beginners/users/magic-link/request`, `GET /_adm/beginners/users/magic-link/display`, `POST /_adm/beginners/users/magic-link/verify` (all public, no auth). |
| Web templates | `templates/web/magic-link-display.html` and `magic-link-display-error.html` already exist (Tera, gateway-rendered HTML pages) — the precedent this feature's new pages will follow. |
| Schema convention | Tables use typed columns + a `meta JSONB` catch-all for future extensibility (`tenant`, `account` — `adapters/diesel_postgres/sql/up.sql:74-102`). No existing "application-wide settings" or "instance state" table. |
| Autogen-secret precedent | Standalone mode's SM-R9 (`feat/standalone-mode`, now merged to `develop`) established a "resolve → keyring → encrypted file → generate-on-first-boot → persist" pattern for secrets that must survive restarts without operator input. Considered and **rejected** for this feature (see DEC-5) — generating+logging a token breaks down across replicas. This feature instead reuses the plain `SecretResolver` config pattern already used for every other gateway secret (`token_secret`, SMTP credentials, etc.): the operator supplies the bootstrap secret via config, identical across all replicas. |
| Dual-adapter requirement | Any new table must exist in both `adapters/diesel_postgres` and `adapters/diesel_sqlite` (confirmed present on `develop`, standalone-mode already merged) — same ~44-port-trait mirroring pattern used for every other repository. |
| REST module layout | `ports/api/src/rest/staff/` is **authenticated**, staff-only (account privilege upgrade/downgrade) — not usable for a pre-staff, unauthenticated bootstrap flow. `ports/api/src/rest/role_scoped/beginners/` hosts the public magic-link endpoints. Neither is the right home for this feature's own routes (see Design §REST layout). |

---

## 3. Decisions (resolved gray areas)

| ID | Decision | Rationale |
|---|---|---|
| DEC-1 | Keep `mycelium-cli accounts create-seed-account` **unchanged** as a manual/dev fallback. | Explicit user instruction — do not touch the existing command. |
| DEC-2 | New flow is a **gateway-served web page** (Tera, like the magic-link display page), not a new CLI subcommand and not a caller of an external API. | User requirement: "totalmente autônomo, que não dependa de interfaces externas... porém continuar usando o cli" — resolved as: the CLI stays as today's fallback; the *autonomous* path is the gateway serving itself, no external caller needed. |
| DEC-3 | **Revised (2026-07-13):** `instance_settings` is a **generalized key/value settings table**, not a table shaped around the bootstrap claim. Each row is one named configuration entry (`key VARCHAR PRIMARY KEY`, `value JSONB`); the shape of `value` is defined and validated entirely by the application/core layer, never by the schema. The staff bootstrap flag is just the first consumer: a row keyed `staff_bootstrap` whose *presence* means claimed and whose *absence* means pending — no `status` column anywhere to drift out of sync with reality. | User feedback: the original design ("singleton row with fixed `staff_bootstrap_status`/`claimed_by_user_id`/`claimed_at` columns, `meta JSONB` as a side reservation") was purpose-built for one concern instead of being the reusable, general-purpose settings store the user actually asked for in the original brainstorming turn ("um modelo que poderá ser usado para outras configurações também sobre a própria instância do mycelium"). Table name itself stays approved ("pode ser esse nome"). |
| DEC-4 | The initial staff **email is typed by the operator on the bootstrap page itself** — not pre-configured in `config.toml`/env. | Confirmed by user. Simpler: no new config surface, no risk of a stale/wrong email baked into an image. |
| DEC-5 | The "proof you are the deployer" credential is an **operator-supplied secret**, configured like every other gateway secret (`SecretResolver<String>` — `Value`/`Env`/`Vault`), compared directly against the presented value at request time. **No token is generated, no hash is stored in the DB.** | Rejected two alternatives after user review: (a) generating+logging a random token at boot — breaks down across replicas (only the replica that wins the row-creation race logs the real value; operators would have to hunt across replica logs); (b) pre-authorizing a fixed staff email in config — forces a redeploy any time the intended admin email needs to change *before* anyone claims it. The config-secret approach keeps DEC-4 (email still typed live on the page) while every replica reads the *same* secret, so there is nothing to hunt for and nothing to redeploy over a typo'd email. |
| DEC-6 | **Revised (2026-07-13):** Multi-replica safety concerns only the **`staff_bootstrap` key's existence**, not the secret (which lives in config, identical across replicas). Claiming *is* creating the row: **`INSERT ... ON CONFLICT (key) DO NOTHING`**. Whoever's insert actually lands wins the claim; everyone else observes `NotCreated` and the winner's value. There is no separate CAS-update step and no boot-time pre-insert — row presence alone carries the state, so a plain atomic insert is the entire compare-and-swap. No advisory locks needed. | Simplification discovered while addressing the DEC-3 revision: once presence-of-row means claimed, the earlier two-step design (idempotent boot-time insert defaulting to `pending`, then a CAS `UPDATE` on claim) collapses into one atomic insert attempt at claim time. Same safety property (well-established pattern for keyed "application settings" rows, e.g. GitLab/Discourse), fewer moving parts. |
| DEC-9 | If `staff_bootstrap_secret` is **not configured**, all `/_adm/instance/bootstrap*` endpoints MUST return 404 unconditionally, regardless of `instance_settings.status`. Bootstrap is opt-in. | Prevents an existing deployment from silently gaining a new public endpoint on upgrade unless the operator deliberately sets the secret. Also the natural "I only want the manual CLI path" escape hatch (DEC-1). |
| DEC-7 | The bootstrap flow **reuses the existing `request_magic_link`/`verify_magic_link` core use-cases unchanged** as building blocks; all bootstrap-specific behavior (token validation, Staff account creation, closing the singleton) lives in new port-layer orchestration and one new small core use-case (`claim_staff_bootstrap`). | Minimizes surface area touched; magic-link core logic and its tests stay untouched, matching the monorepo's "prefer extending existing ones" dependency/reuse rule. |
| DEC-8 | Branching: gateway submodule branch created from `develop` (`feat/staff-bootstrap`, since `feat/standalone-mode` is already merged there); monorepo pointer work targets `main`, not the currently-checked-out unrelated `fix/webapp-dokploy-node22` branch. | Explicit user instruction. |

---

## 4. Functional requirements

Each requirement has a traceable ID for design/tasks.

### Instance state

- **SB-R1** — A new generalized key/value table `instance_settings` (Postgres **and** SQLite
  adapters) MUST store: `key` (primary key), `value` (JSON), `created_by`/`updated_by` (JSON,
  nullable, holding a `WrittenBy` — see SB-R13), `created`/`updated` timestamps. The shape of
  `value` is owned entirely by whichever core-layer concern uses a given key — the schema itself
  knows nothing about `staff_bootstrap` or any other specific setting. No secret/token material is
  stored in this table (see DEC-5).
- **SB-R2** — The staff bootstrap claim is keyed `staff_bootstrap` (`STAFF_BOOTSTRAP_KEY`). Its
  *presence* means claimed; its *absence* means pending. Nothing is pre-created at boot — boot only
  **fetches** the key to decide whether to log the reminder (see below). This replaces the earlier
  "idempotent boot-time insert defaulting to pending" design (DEC-6 revision).

### Bootstrap secret configuration

- **SB-R3** — A new optional config field (e.g. `AccountLifeCycle.staff_bootstrap_secret:
  Option<SecretResolver<String>>`) holds the operator-supplied bootstrap secret, resolved the same
  way every other gateway secret is (`Value`/`Env`/`Vault`).
- **SB-R4** — If `staff_bootstrap_secret` is **not configured**, every `/_adm/instance/bootstrap*`
  endpoint MUST return 404 unconditionally, regardless of whether the `staff_bootstrap` key exists
  (DEC-9). Bootstrap is opt-in. On boot, if it **is** configured and the `staff_bootstrap` key does
  **not** exist yet (still pending), the gateway MUST log (structured, `tracing`) an informational
  reminder with the claim URL (`{domain_url}/_adm/instance/bootstrap`) — but never the secret
  itself, since the operator already knows it from their own config.

### Bootstrap claim flow (public, unauthenticated, gateway-rendered)

- **SB-R5** — `GET /_adm/instance/bootstrap`: if bootstrap is disabled (SB-R4) or the
  `staff_bootstrap` key already exists (already claimed), renders an error page (410-style, reusing
  the existing `*-display-error.html` pattern) or 404. Otherwise renders a Tera page with a secret
  input **and** an email input (DEC-4 — the operator types both; nothing is pre-filled or carried
  as a hidden field).
- **SB-R6** — `POST /_adm/instance/bootstrap/request-code { secret, email }`: validates `secret`
  against the configured value **and** confirms the `staff_bootstrap` key doesn't exist yet (SB-R4),
  then calls the **existing, unmodified** `request_magic_link` use-case for the typed email. Same
  generic `{ sent: true }` response regardless of outcome (no enumeration).
- **SB-R7** — `POST /_adm/instance/bootstrap/complete { secret, email, code }`: re-validates
  `secret` + key-not-yet-claimed, calls the **existing, unmodified** `verify_magic_link` use-case
  (creates the `User` if absent), then — only on success — calls the new `claim_staff_bootstrap`
  core use-case. Claiming *is* creating the row: `INSERT INTO instance_settings (key, value) VALUES
  ('staff_bootstrap', {claimedByUserId, claimedAt}) ON CONFLICT (key) DO NOTHING`, attempted
  *before* touching the `Account` repo at all. Only if this insert actually lands (this call won the
  key) does it then create/upgrade the `Account` to `AccountType::Staff` for that user. If the
  insert is a no-op (lost a race to a concurrent claim — the key already existed), return an
  "already initialized" error immediately — the Account repo is never called, so there is nothing
  to roll back (see Design D-1).
- **SB-R8** — Once the `staff_bootstrap` key exists, all three endpoints above MUST return
  404/410 unconditionally — the secret alone is never sufficient once the key exists (existence is
  checked on every request, not just the secret).

### Consistency with the manual CLI path

- **SB-R9** — If the manual `accounts create-seed-account` CLI path (DEC-1) is used instead of the
  web flow, it MUST also best-effort create the `staff_bootstrap` key (same `get_or_create` call the
  web flow's claim uses) so the web bootstrap endpoints correctly disable themselves afterward. A
  failure to write `instance_settings` here MUST NOT fail the CLI command (the seed account is the
  source of truth; the entry is best-effort bookkeeping) but MUST be logged.

### Non-regression

- **SB-R10** — `request_magic_link` and `verify_magic_link` core use-cases and their existing tests
  MUST remain byte-for-byte unchanged.
- **SB-R11** — Full-mode gate checks MUST pass unchanged: `cargo fmt --all -- --check`, `cargo build
  --workspace`, `cargo test --workspace --all`. Standalone build (`--no-default-features --features
  standalone`) MUST also build and pass its own tests with the new table present.
- **SB-R12** — Postgres migrations in this project are operator-applied (verified: no script or
  Dockerfile applies `sql/migrations/*.sql`, and the existing envelope-encryption migration's
  columns are not folded into `up.sql` either — same manual convention this feature follows). If an
  operator boots a new gateway binary before applying this feature's migration, boot MUST NOT
  panic: `staff_bootstrap_is_pending`'s fetch failures are logged and swallowed, and the gateway
  starts normally without the bootstrap log line (unlike the hard-fail-at-boot posture for
  Postgres/Redis/SMTP, this table backs an optional, opt-in feature).
- **SB-R13** — (added 2026-07-13, user request) `claim_staff_bootstrap` MUST record who performed
  the claim in `instance_settings.created_by`, using the shared `WrittenBy` DTO
  (`core/src/domain/dtos/written_by.rs`). `WrittenBy` MUST be generalized to also work when no
  User/Account id exists yet: `id`/`from` become optional, and an optional base64-encoded `email`
  field is added, independent of `id`/`from`. When only an email is known (no id at all — the
  degenerate case this feature motivates), `WrittenBy` MUST still be informative
  (`email:<base64>`), not empty. Pre-existing `created_by`/`updated_by` JSON records on other
  tables (`account`, `webhook`) that predate the `email` field MUST continue to deserialize
  correctly (`#[serde(default)]`) — no retroactive data migration.

---

## 5. Out of scope

| Item | Reason |
|---|---|
| Pre-configuring the initial staff email via `config.toml`/env | DEC-4 — typed on the page instead. |
| Reusing/extending SM-R9's keyring/file secret mechanism | DEC-5 — a plain `SecretResolver` config value is simpler and avoids the multi-replica log-hunting problem. |
| Expiring/rotating the bootstrap secret by time (TTL) | Not required for MVP; the secret is single-use-until-claimed (status-gated, not time-gated) — DEC-9 already closes it permanently on claim. Rotation, if ever needed, is a config change + restart, same as any other secret. |
| Creating a first `Tenant` as part of bootstrap | Staff accounts are not tenant-scoped in this system; tenant creation is an unrelated, already-existing flow. |
| Changing `create_seed_staff_account` behavior or signature | DEC-1 — untouched. |
| Rate limiting the bootstrap endpoints | Not implemented anywhere in the gateway yet (roadmap M4); same posture as every other public endpoint today. |

---

## 6. Open concerns (carry into design)

- **OC-1** — Orphaned `User` row if `verify_magic_link` succeeds (SB-R7) but the subsequent CAS
  claim loses the race: the email now has a plain, non-Staff `User` with no `Account`. Low severity
  (identical to any ordinary magic-link signup), but confirm this is an acceptable, documented edge
  case rather than something the design needs to actively clean up.
- **OC-2** — **Resolved (D-1, design.md):** no cross-repo transaction exists anywhere in this
  codebase (verified against `create_seed_staff_account`'s precedent — two independent,
  non-transactional repo calls). `claim_staff_bootstrap` attempts the `staff_bootstrap` key insert
  before the Account write, so a lost race (key already exists) never touches the Account repo at
  all. Residual, accepted risk: the insert succeeds but the following Account-upgrade call fails —
  instance ends up permanently "claimed" with no Staff account. Recoverable via the manual CLI path
  (SB-R9), since `create_seed_staff_account` doesn't check `instance_settings` at all.
- **OC-3** — Whether the bootstrap claim URL logged at boot (SB-R4) must go through the same
  `life_cycle_settings.domain_url` resolution used by `request_magic_link` today (very likely yes,
  per `gateway-architecture.md`'s rule that use-cases never build URLs — port layer must build and
  inject it, same as the existing magic-link pattern).
- **OC-4** — **Resolved (design.md):** `staff_bootstrap_secret` comparison uses
  `subtle::ConstantTimeEq`, mirroring `lib/http_tools/src/telegram/verify_webhook_secret.rs`'s
  existing pattern exactly. `ring::constant_time::verify_slices_are_equal` was considered and
  rejected — it's deprecated in the pinned `ring` version with an explicit "not intended for
  external use, no promises regarding side channels" note.

---

## 7. Acceptance criteria

- [ ] A fresh full-mode deploy (Postgres, no pre-existing staff, `staff_bootstrap_secret`
      configured) boots, logs a bootstrap URL reminder (no secret in the log), and an operator who
      knows the configured secret can complete the entire flow from a browser with zero DB/API
      access.
- [ ] A deploy with no `staff_bootstrap_secret` configured never exposes the bootstrap endpoints
      (404 unconditionally), regardless of whether the `staff_bootstrap` key exists.
- [ ] Two operators racing to claim concurrently converge on exactly one winning `staff_bootstrap`
      row (no split-brain, no duplicate rows, no lost update) — the loser's request is rejected
      cleanly with no Account created.
- [ ] After claiming, the bootstrap endpoints are permanently inert, even if the correct secret is
      replayed.
- [ ] `instance_settings.created_by` for the `staff_bootstrap` row contains the claiming user's id
      and base64-encoded email (`WrittenBy`), readable without any DB-side decoding.
- [ ] `accounts create-seed-account` behavior is unchanged; using it also closes the web bootstrap
      path, recording the same `WrittenBy` shape in `created_by`.
- [ ] `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all` all
      pass; standalone build unaffected.
- [ ] `request_magic_link`/`verify_magic_link` source and tests show zero diff.
