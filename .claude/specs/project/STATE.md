# State

**Last Updated:** 2026-08-28
**Current Work:** **Release hygiene.** Issue #187 (parentless guest role grant,
`features/parentless-guest-role-grant/`, AD-011) is **merged** into `develop` via PR #189.
Follow-up in flight: the `9.0.0` release commit never came back from `main`, so `develop` still
declared `9.0.0-rc.13` — see AD-012.

**Also open:** **9.0.0 stable release prep.** Single-file Postgres install
(`features/single-file-postgres-install/`, AD-010) implemented and verified on Postgres
12/14/16, **awaiting user UAT before commit** — `up.sql` is now the complete 9.0.0 schema plus
a new migration fixing a grant bug the fold exposed. Latest tag is `9.0.0-rc.12`; **open
question with the user:** cut rc.13 first or go straight to stable. Either way
`release-stable.yml` requires `refs/heads/main`, so a `develop` → `main` PR is a prerequisite.

**Previously:** Two header trust-boundary fixes in the gateway router, both implemented in the
same working tree on branch `fix/gateway-response-header-blocklist`, all gates green (449 tests),
**awaiting user UAT before commit** — response-side (`features/gateway-response-header-blocklist/`,
AD-008) and request-side (`features/strip-inbound-mycelium-headers/`, AD-009, **security fix**).
See the commit plan in the second spec: disjoint file sets, meant to become two commits/PRs.
Local Email DX (`features/local-email-dx/`) — implemented on branch
`feat/stub-pretty-render-and-file-transport`, all gates green, **awaiting user UAT before commit**
(see AD-007). Resource Audit Log — spec/design/tasks written 2026-07-13, awaiting user go-ahead to
Execute (see block below). Standalone Mode G1-G9 done, not committed yet. M1 ongoing.

---

## Recent Decisions (Last 60 days)

### AD-012: the stable release commit must be back-merged into `develop` (2026-08-28)

`cargo release` bumps the version **on the branch it runs from**. `9.0.0` was cut on `main`, so
`chore: Release version 9.0.0` (`cdd8f8bc`) lived only there; `develop` still declared
`9.0.0-rc.13`. `release-prerelease.yml` runs from `develop`, so a dispatch with `beta` read the
stale version and died on cargo-release's misleading `unsupported release level beta, only major,
minor, and patch are supported` (run 33205154348).

The message is a red herring: `src/ops/version.rs`'s `increment_beta` rejects the **rc → beta**
transition specifically. Confirmed by diffing cargo-release 1.1.3 (last green run) against 1.1.5
(the failing one) — that code is byte-identical, so the tool upgrade was not the cause.

**The dangerous case is not the one that failed.** With `develop` behind, a `rc` dispatch would
have silently cut `9.0.0-rc.14` — a version *behind* the already-published `9.0.0` — and shipped
it to crates.io and GHCR without a single error.

**Decisions:**

- **Back-merge is a mandatory step after every stable release**, documented in
  `docs/book/src/08-release-process.md` (it was not mentioned anywhere before) and added to the
  release checklist.
- **`release-prerelease.yml` gained a pre-flight step**, placed before the ~6min cargo-release
  build so a bad dispatch fails in seconds. It refuses to run while `develop`'s version is lower
  than `main`'s, and it explains the rc → beta rejection in its own words.
- **The guard compares versions, not commit ancestry**, so it is satisfied whether a back-merge PR
  is merged or squashed.
- **`sort -V` is not semver.** It ranks `9.0.0-rc.13` *above* `9.0.0`. The comparator rewrites the
  pre-release hyphen as `~` first, which GNU sort ranks before everything — verified against
  rc/beta/stable/minor pairs.

### AD-011: a root guest role is grantable by whoever already holds it (2026-08-28)

**Feature:** `features/parentless-guest-role-grant/`. Closes issue #187.

`guest_to_children_account` required the granted role to have a parent, which made every **root
role undelegable by anyone** — `get_parent_by_child_id` returns NotFound and the call dies at
MYC00013. The data-side workarounds are all closed: `insert_role_child` requires
`parent.permission >= child.permission`, `get_or_create` de-duplicates on `(slug, permission)`,
and `Permission` stops at `Write = 1`, so a same-slug parent is inexpressible; a different-slug
parent authorizes nothing downstream because `LicensedResource.role` carries the **slug** and
consumers match it by equality.

**Decisions:**

- **Branch on the parent instead of requiring one.** Parent found → the existing rules (hold the
  parent, target listed in `parent.children`). Parent absent → the requester must already hold
  the target role itself. The invariant that matters is preserved: nobody grants what they do
  not have.
- **The root path is scoped, the parent path is not.** The new possession check runs on
  `profile.on_tenant(tenant_id).on_account(target_account_id)` so holding the role on one account
  cannot authorize granting it on another. The pre-existing parent check keeps reading the full
  license list — rule (1) forces `accounts-manager` on the *target* account but nothing
  guarantees the requester holds the *parent role* there, so tightening it could break live
  grants. Deliberate asymmetry; **follow-up candidate** once the production topology is
  confirmed.
- **Resolve the target role before the parent branch.** A nonexistent role has no parent either,
  so the old order would route a missing role into the root path and report it as a missing
  license.
- **Staff/manager stay strict.** `get_related_account_or_error` short-circuits on `is_staff` /
  `is_manager` without touching `licensed_resources`, so such a profile clears rule (1) and then
  fails the possession check. That is today's behavior on the parent path; kept.
- **Rejected: a third `Permission` level (`Admin = 2`).** `Permission::from_i32` maps
  `_ => Read`, so any not-yet-updated service would silently read `2` as read-only.

**Coverage:** the use case had **no tests**. Seven added in-file (root held / not held / held on
another account, child with parent held / not held / not in children, missing target role).

**Out of scope:** the webapp needs no change — `GuestRoleSelector` does not filter by hierarchy.
The SDK slug-equality helper from the issue's *Related finding* is a separate ticket.

### AD-010: `up.sql` is the complete Postgres schema; folding is mandatory (2026-08-12)

**Feature:** `features/single-file-postgres-install/`. Blocks the 9.0.0 stable release.

Postgres was the only backend without a one-shot install: `up.sql` + five hand-applied
migrations, three of them not re-runnable. SQLite self-installs (`embed_migrations!` +
`provision_database`, called from `main.rs:112`), so it needed nothing.

**Decisions:**

- **Fold into `up.sql`; no sibling `install.sql`** (user decision). This reverses the
  convention in `staff-bootstrap/design.md` and `postgres-only-mode/spec.md` — but that
  convention was **already broken**: `kv_artifact` and `idx_message_queue_claim` had been
  folded in during postgres-only-mode, matching that feature's `tasks.md:104` and
  contradicting the other two docs. Only 3 of the 5 migrations were actually missing.
  New rule recorded in `CONVENTIONS.md`: a migration goes in `sql/migrations/` **and** is
  folded into `up.sql` in the same commit.
- **Two-phase file.** `CREATE DATABASE` can't run in a transaction and `\c` reconnects, so
  phase A (vars, database creation, guard) is unwrapped and phase B (all DDL) is a single
  `BEGIN`/`COMMIT`. Verified atomic: a bogus column type leaves 0 tables behind.
- **Re-runnability via an early-exit guard, not 40 `DO` blocks.** Postgres has no
  `ADD CONSTRAINT IF NOT EXISTS`; probing `to_regclass('public.account')` and `\quit`ing
  with a "use sql/migrations/ to upgrade" message achieves the same in three lines.
  `CREATE ROLE`/`CREATE USER` *are* individually guarded — roles are cluster-global, so an
  unguarded `CREATE ROLE` fails on the second Mycelium database in a cluster, a case the
  early-exit guard doesn't cover. An existing user keeps its password.
- **`CREATE INDEX CONCURRENTLY` → plain `CREATE INDEX`** in `up.sql` (required by the
  transaction; pointless on an empty database). The `CONCURRENTLY` form and the legacy
  `DROP INDEX ... _per_tenant` stay in `sql/migrations/` for in-place upgrades.
- **Embedded Postgres migrations deferred**, not adopted: every replica in a multi-pod
  deployment would race to migrate at boot. Larger design question than the release needs.

**Bug found by the verification diff, not by reading:** `instance_settings` and
`resource_audit_log` have **no grants** on any database built from `up.sql` + migrations.
`up.sql`'s `GRANT ALL ON ALL TABLES` is evaluated at execution time and sits at the end of the
file, so it never covered tables created later by migrations — and unlike `20260722_01`
(`kv_artifact`), neither of those two migrations granted explicitly. Superuser deployments
never notice (including `eva-natural-ai`); a README-style install where the app connects as
`db_user` fails on staff bootstrap and every audit-log write. Fixed by a **new** migration,
`20260812_01_audit_tables_grants.sql` — not by editing the shipped files, since nothing tracks
which migrations ran and an operator would never re-read an old one. **It must be applied by
the table owner** (the superuser that ran `up.sql`): as `db_user` it fails with
`permission denied for table instance_settings`. That is pre-existing convention — the shipped
`20260722_01` fails the same way (`must be owner of table kv_artifact`) — now documented in
both the migration header and `docs/book/src/02-installation.md`.

**Verification:** built both paths (migrated vs consolidated) on throwaway containers and
diffed `pg_dump --schema-only`. Identical — 420 normalised lines — on Postgres **12, 14 and
16**. Plus: guard no-ops on re-run, role reuse works, atomicity holds, all three folded tables
carry grants, immutability trigger still fires. **Deferred:** a CI job asserting the two paths
produce identical dumps is the only durable enforcement of the new convention.

**Status:** implemented, **not committed** (awaiting user UAT per commit-validation rule). No
Rust changed, so cargo gates are untouched by this work.

### AD-009: Client-supplied `x-mycelium-*` headers are stripped before forwarding (2026-07-27)

**Feature:** `features/strip-inbound-mycelium-headers/`. **Security fix — authorization bypass.**

`awc::Client::request_from(url, req.head())` copies *every* client header into the downstream
request. The gateway then overwrote only *some* `x-mycelium-*` headers, and only for certain
security groups (`check_security_group.rs:99-188`): `Public` overwrites nothing, `Authenticated`
overwrites only `x-mycelium-email`. So on any Public or Authenticated route a client could send a
forged `x-mycelium-profile` and the gateway handed it to the downstream service, where every SDK
decodes it as gateway-attested identity — `hasStaffPrivileges` included. `x-mycelium-scope`,
`x-mycelium-role` and `x-mycelium-tenant-id` had **no producer at all** on the proxy path and
passed through in every security group.

**Decisions:**

- Strip is **prefix-based** (`MYCELIUM_HEADER_PREFIX = "x-mycelium-"`, new in `settings.rs`), not a
  list of known constants. A constant list re-breaks the day a new `DEFAULT_*_KEY` is added and the
  filter is not updated — the exact failure mode of both bugs found this session.
- Runs **unconditionally**, inside `initialize_downstream_request` right after `request_from` and
  before every gateway `insert_header`. Not per security group — that enumeration *was* the bug.
- `x-mycelium-request-id` is **explicitly re-injected** after the strip. It was previously forwarded
  only as a side effect of the blanket copy. Safe: the app-level `wrap_fn` (`main.rs:805-812`)
  overwrites it with a fresh uuid on every inbound request before the router runs.
- **`x-mycelium-connection-string` stops being forwarded downstream** — deliberate. It is the
  user's own bearer-equivalent credential. Searched the gateway and all three SDKs: each SDK only
  *declares* the constant, none reads the header. Services outside the monorepo can't be checked —
  flagged for UAT; if one does read it, re-inject it explicitly like the request id.
- `x-mycelium-security-group` is stripped too, but `check_security_group.rs:61-62` re-inserts it
  unconditionally in a later pipeline step, so downstream still receives it.
- Pure `fn(&mut HeaderMap)` in its own file (`strip_inbound_mycelium_headers.rs`), so it is
  unit-testable without constructing an `awc::Client`.

**Reclassifies `.claude/specs/security/findings.md` §3.1**, which recorded profile spoofing as
conditional on the downstream service being exposed *without* the gateway. It was reachable
*through* the gateway — the path the SDK contract treats as trusted.

**Out of scope (user decision):** profile signing (HMAC gateway↔SDK) and trusted-proxy validation
in `resolve_client_ip` (findings.md §1.3).

**Testing:** 6 unit tests on the pure function. They are **not** regression tests — the function is
new, so none would have caught the original bug, which was its absence from the pipeline. A
pipeline-level test (forged profile through `initialize_downstream_request` on a `Public` route)
was proposed and **declined by the user: unit tests only**. Strip point and ordering verified by
reading `router/mod.rs:154-197` and `check_security_group.rs:61-62`.

**Gates:** all green — `fmt --check`, `build --workspace`, `test --workspace --all` (449 tests,
6 new). **Status:** implemented, **not committed** (awaiting user UAT per commit-validation rule).

### AD-008: Gateway response headers are a blocklist, not an allowlist (2026-07-27)

**Feature:** `features/gateway-response-header-blocklist/`. Branch
`fix/gateway-response-header-blocklist`.

`build_the_gateway_response` kept **only** the headers present in a short list
(`route_key` + `FORWARDING_KEYS` + forwarded-for + profile + service-name), inverting the
documented intent of `FORWARDING_KEYS` (`settings.rs`: "headers that should be removed"). Two
consequences: every legitimate downstream header was dropped (`Content-Type`, `Location`,
`Cache-Control`, `ETag`, `Content-Disposition`, `Set-Cookie`, app-custom), and the only headers
guaranteed to pass were the ones that must not — `route_key` is the **name of the injected
downstream secret header**, so an echoing downstream leaked it to the client.

**Decisions:**

- Blocklist is split into two *named* sets: hop-by-hop (`FORWARDING_KEYS`, already the complete
  RFC 7230 §6.1 set — **nothing new added**) and gateway-injected artifacts (`route_key`,
  `x-mycelium-profile`, `x-mycelium-service-name`, `x-forwarded-for`, `x-mycelium-request-id`).
- `x-mycelium-request-id` in the second set also kills an ordering hazard: the gateway inserts its
  own id first, and an echoed downstream value would otherwise overwrite it.
- **`Content-Encoding` and `Content-Length` are deliberately NOT blocked.**
  `initialize_downstream_request.rs` calls `.no_decompress()`, so the body reaches the client still
  compressed — stripping `Content-Encoding` would corrupt every gzip response. `Content-Length` is
  already dropped by the actix h1 encoder for `BodySize::Stream` but is intentionally retained for
  `304` (RFC 7232 §4.1).
- Signature changed to `(request_id, route_key, StatusCode, &HeaderMap)` — the function only ever
  read status and headers, and `awc::test::TestResponse` yields the wrong payload type to build a
  `DownstreamResponse` in tests.

**Blast radius:** `route_request` is only `App::default_service`, so this affected 100% of proxied
traffic and 0% of Mycelium's own handlers (which set their own Content-Type via `Responder`) —
that split is why it went unnoticed. Nothing masked it: no `Compress`, no `DefaultHeaders`
middleware, and actix never supplies a default `Content-Type` for streamed bodies. Proxied JSON has
been served with no `Content-Type` all along; SSE was just the first client strict enough to care.

- The copy loop must use **`append_header`**, not `insert_header`: `HeaderMap::iter` yields one
  entry per value, so an insert collapses multi-valued headers (`Set-Cookie`, `Vary`, `Link`) to
  their last value. Only reachable once the allowlist is inverted — caught in review, verified by a
  test that fails on `insert_header`.

**Follow-up (same PR, 3rd commit):** the first pass *enumerated* the Mycelium keys to block
(profile, request-id, service-name) and so still leaked `x-mycelium-email`,
`x-mycelium-security-group`, `x-mycelium-connection-string`, `x-mycelium-scope`,
`x-mycelium-role` and `x-mycelium-tenant-id` back to the client if a downstream echoed them —
authenticated user's email, the route's authorization config, and the user's connection-string
credential. Caught by the automated security review of the commit, **not** by the tests. Now
matched by `MYCELIUM_HEADER_PREFIX`, same rule as the request side. Lesson is already recorded in
`settings.rs`: never enumerate the namespace, and it was violated in the very next file.

**Testing:** 8 tests, all verified as regression tests — the filter was temporarily reverted to the
original allowlist and the suite re-run (0 passed / 7 failed), then restored (7 passed). T8
verified separately against the enumerated blocklist (failed → passed).

**Gates:** all green — `fmt --check`, `build --workspace`, `test --workspace --all` (7 new tests).
**Status:** implemented, **not committed** (awaiting user UAT per commit-validation rule).

### AD-007: Local Email DX — stub terminal render + file-transport wiring (2026-07-17)

**Feature:** `features/local-email-dx/` (spec/design/tasks). Branch
`feat/stub-pretty-render-and-file-transport`. Two independent threads, one PR:

- **Thread A (stub render):** the standalone stub transport now renders each undelivered email as a
  bordered, human-readable block to **stdout** via `println!` (not `tracing` — deliberately kept out
  of structured logs/SigNoz, DEC-1). HTML→text via the new **`html2text`** workspace dep, compiled
  only under the notifier's `local-transport` feature (never in full). Renderer lives in
  its own file `render_stub_email_for_terminal.rs`; links surfaced at the top, copyable.
- **Thread B (issue #169):** new opt-in `[localEmail.define] dir = "..."` config
  (`OptionalConfig<LocalEmailConfig>`, standalone-only), wired through `ConfigHandler` into
  `select_local_transport(smtp, file_dir)` in `main.rs`, replacing the hardcoded `None`. Precedence
  unchanged (SMTP > File > Stub). Enable-shape is `.define` (OptionalConfig external tag), same as
  `[auth.internal.define]`. **Gotcha caught in review:** lettre's `FileTransport::new` does *not*
  create the target dir (it `fs::write`s per send and errors if missing) — so `LocalEmailConfig`
  gained `ensure_dir()` (create_dir_all), called from `main.rs` before wiring, matching sqlite's
  "created on first boot" convention.

**Gates:** all green — `fmt --check`, workspace build+test (29 bins), standalone build+test (api 24,
notifier local-transport 14), `html2text` confirmed absent from the default build tree.
**Status:** implemented, **not committed** (awaiting user UAT per commit-validation rule).
CHANGELOG left to git-cliff (conventional commit will drive it).

### AD-006: Resource Audit Log — scope, permission model, async mechanism (2026-07-13)

**Decision:** New immutable `resource_audit_log` table records every create/update/delete on
mycelium's own resources (**all domains except `error_code`**, confirmed with the user): `account`
(covers every `AccountType` subtype including subscription/role-associated/tenant-manager —
"subscription account" is not a separate resource_type, it's an `account` row with `tenant_id`
populated), `account_meta`, `user`, `tenant`, `tenant_meta`, `guest_role`, `webhook`. Never audits
downstream-service traffic (gateway-proxied requests) — mycelium-internal events only.

**Read permission rule:** staff always; tenant-scoped resources → tenant owner/manager; personal
resources → the resource's own account owner; global resources (`webhook`, no tenant/owner) →
staff only. Confirmed explicitly with the user rather than assumed.

**Async write mechanism:** bounded `tokio::sync::mpsc::channel` + a single background consumer
task (new `resource_audit_log_dispatcher`, spawned in `main.rs` next to the existing
`webhook_dispatcher`), not spawn-per-event. The port's `create()` does a non-blocking `try_send`
and always returns `Ok(())` — audit failures are only traced, never propagated to the caller.
Chosen over spawn-per-event specifically to preserve insertion order and avoid audit writes
competing with request-path code for the same r2d2 pool.

**Immutability:** two independent layers — no `Updating`/`Deletion` port exists in code (only
`ResourceAuditLogRegistration` + `ResourceAuditLogFetching`), AND a DB trigger rejects
`UPDATE`/`DELETE` regardless of who connects (code-only enforcement is bypassable via direct DB
access, and the app's DB role likely owns the table so `REVOKE` alone would be illusory).

**Index for the dominant read** (`WHERE resource_id = ? ORDER BY created_at DESC`):
`(resource_id, created_at DESC)` — deliberately NOT led by `resource_type`, since `resource_id`
(a UUID) alone already narrows correctly; `telegram_identity_audit`
(`adapters/diesel_postgres/sql/up.sql:417-431`) is the closest existing schema/index precedent,
though it's dead SQL today (no Diesel model/consumer).

**Backend parity requirement:** both `myc_diesel` (Postgres) and `myc_diesel_sqlite` (standalone)
need the port implementation — `active_backend_modules.rs` cfg-gates between them, so a
postgres-only port would break the `standalone` feature build.

**Spec:** `.claude/specs/features/audit-log/` (spec.md + design.md + tasks.md — 52 tasks; P1 =
`account` domain vertical slice, P2 = `tenant`+`guest_role`, P3 = `user`+`webhook`+
`account_meta`+`tenant_meta`).

**Execute — done, 2026-07-13.** All 52 tasks implemented via ~20 parallel/sequential subagent
dispatches (foundation → adapters/dispatcher/main.rs wiring → 7 instrumentation lanes + REST/RPC
endpoint). Final gate, all green: `cargo fmt --all -- --check`, `cargo build --workspace`,
`cargo build -p mycelium-api --no-default-features --features standalone`, `cargo test --workspace
--all` (331 `myc-core` tests + all other crates, 0 failed).

**Committed and PR opened as draft, 2026-07-13** (user explicitly authorized commit): new branch
`feat/resource-audit-log` off `origin/develop` (the working tree was on `feat/staff-bootstrap`,
which turned out to already be fully merged into `develop` via PR #168 — created a clean new
branch instead of stacking on an already-merged one). Commit `ff88a951`, 122 files, +11473/-218.
PR **#170** → `develop`, draft: https://github.com/LepistaBioinformatics/mycelium/pull/170.
Monorepo submodule pointer NOT updated yet — deliberately deferred until the PR is out of draft/
merged, per the usual pointer-update-after-merge convention.

**Correctness fix found and applied mid-execution:** the spec's edge case "only confirmed
successful mutations are audited" was violated in 5 of the 41 instrumented use cases (all in the
`*_meta` domains) — `emit_resource_audit_event` was called unconditionally after `?`, so an
`Ok(NotCreated/NotUpdated/NotDeleted)` response (the write did NOT happen) still logged a
`Created`/`Updated`/`Deleted` event. Fixed by gating the emit call inside the genuine success match
arm only; a dedicated sweep confirmed all other 36 files were already correct.

**Incident found and resolved mid-execution:** one of ~10 concurrently-dispatched subagents ran
`git stash` on this shared (non-worktree) working tree to "isolate" its own testing, hiding ~4000
lines of 5 other lanes' uncommitted work. Diagnosed via `git diff stash@{N} -- <file>` per file;
24/26 stashed files were byte-identical to the working tree (redundant, safely dropped), 3 files
(user lane) were genuinely reverted and restored via `git checkout stash@{N} -- <file>`, 2 files
(guest_role/webhook) had independently-recreated duplicate instrumentation and the working-tree
version was kept. Stash dropped once fully reconciled. **Standing rule going forward: any
multi-agent dispatch that edits a shared working tree must explicitly forbid `git stash`/`checkout
--`/`reset` in every agent prompt** — see auto-memory `feedback_no_git_stash_in_parallel_agents`.

### AD-005: Standalone mode is a compile-time feature, not a runtime `mode` flag (2026-07-06)

**Decision:** Standalone (SQLite + moka + stub/file email + autogen secrets) is selected by a
`standalone` cargo feature, mutually exclusive with the default `full`, producing a
separate binary/image. The roadmap's original `mode = "standalone"` runtime flag is rejected.

**Reason:** Every table's schema uses Postgres-only Diesel SQL types (`Uuid`, `Timestamptz`, `Jsonb`,
`Array<Text>`, `Array<Jsonb>`). These do not compile against Diesel's SQLite backend. A single
runtime-switchable binary would require storing them as TEXT in Postgres too, changing full mode
(violates "don't break full"). `MultiConnection` (Diesel 2.3.7) cannot span `Array`/`Jsonb` either.

**Trade-off:** Two binaries to build/ship; a parallel SQLite schema + model set + SQLite variants of
~44 repository impls (the dominant cost — close to a second repository-layer implementation).

**Corrections captured in spec:** (a) no Redis pub/sub exists — the only queue is an LPUSH email
list; (b) no rate-limiting / AI-retry-loop detection exists (brief limitation #1 is moot); (c) token
invalidation is DB-backed, not Redis (limitation #3 overstated); (d) `mem_db` does not implement the
KV traits, so a new `moka` adapter is required; (e) today there are 3 hard boot deps (PG+Redis+SMTP),
not "single dependency (PostgreSQL)".

**Secrets:** auto-generate once and persist (keyring-preferred, encrypted-file fallback). Regenerating
`token_secret` per restart would rotate the KEK and invalidate all connection strings (see AD-004).
Open concern: keyring is usually unavailable on the container/air-gapped/edge targets, so the file
fallback is the de-facto primary path.

**Spec:** `.claude/specs/features/standalone-mode/` (spec.md + design.md + tasks.md — 27 tasks in 8
groups; critical path is G2 SQLite backend). No implementation yet. All work is on the dedicated
branch **`feat/standalone-mode`** (gateway submodule) — never `develop` (user instruction 2026-07-06).


### AD-004: HMAC signing key decoupled from `token_secret`; KVR-versioned, no legacy fallback (2026-04-25)

**Decision:** Connection-string HMAC signing now uses a dedicated `hmac_secret` set keyed by
explicit version (`hmacPrimaryVersion` + `[[hmacSecrets]]`). The `KVR(u32)` bean is part of
the HMAC input (anti-downgrade). Tokens missing or carrying an unknown KVR are rejected
(`MYC00030` / `MYC00031`); there is **no implicit "missing KVR = v1"** fallback.

**Reason:** `token_secret` previously played two roles — KEK source (envelope encryption) and
HMAC signing key for connection strings. Rotating it atomically revoked every live connection
string, which is unacceptable when operators want to rotate the KEK or the signing key
independently. Versioning the HMAC key set makes rotation a routine operation: add a new
version, bump primary, wait for TTL drain, retire the old version.

**Trade-off:** **BREAKING.** Etapa 3 deploy permanently invalidates connection strings issued
under the old single-secret scheme — every active user must re-authenticate on next request.
The deployment warning is reproduced in three places: `settings/config.example.toml`, the
operator runbook (`docs/book/src/22-hmac-key-rotation.md`), and the changelog entry. Operator
must plan the rollout during a window where full re-auth is tolerable.

**Impact:** New CLI command `myc-cli rotate-kek` lets operators rotate the KEK without
touching connection strings (re-wraps every DEK in place; idempotent). Verification runs
**before** the DB lookup in `fetch_connection_string_from_request` middleware via
`Mac::verify_slice` (constant-time). New error codes: `MYC00030` (MissingKeyVersion),
`MYC00031` (UnknownKeyVersion), `MYC00032` (SignatureMismatch).

**PR:** #151 merged 2026-04-25 (4 commits: `d4e21e94`, `2a6ebfbf`, `b2d4685f`, `39151012`).
Spec at monorepo `.claude/specs/features/hmac-key-rotation/`.

---

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

### AD-003: Per-tenant secrets use AES-256-GCM encrypted at rest, not SecretResolver (2026-04-19)

**Decision:** Secrets that vary per tenant (Telegram bot token, webhook secret) are stored as
`base64(nonce ‖ AES-256-GCM ciphertext ‖ tag)` in the `tenant.meta` JSONB column. The encryption
key is derived from `AccountLifeCycle::token_secret` via SHA-256 (`derive_key_from_uuid`).
`SecretResolver<String>` is not used for this class of secrets.

**Reason:** `SecretResolver` requires the operator to format the stored value as JSON
(`"\"plain-token\""` for plain text, `{"env":"VAR"}` for env, `{"vault":{…}}` for Vault).
This is not documented in the field names and causes silent failures at runtime when the format
is wrong. Encrypted at rest gives a uniform, operator-friendly write path (plain string in,
ciphertext stored) with no format ambiguity.

**Trade-off:** If `AccountLifeCycle::token_secret` rotates, all per-tenant secrets encrypted
under the old key must be re-submitted via the config endpoint. No automatic re-encryption.

**Pattern to follow for future per-tenant secrets:**
- Write: call `encrypt_string(&plain, &config)` from `core::domain::utils` in the use case
- Read: call `decrypt_string(&ciphertext_b64, &config)` in the adapter constructor (eagerly)
- Store under a `TenantMetaKey` variant with a descriptive name (no `Ref` suffix)

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

### L-002: Personal accounts vs subscription accounts — Telegram IdP model (2026-04-19)

**Context:** The original Telegram IdP spec (OQ-2b) stored identity on subscription accounts
(tenant-scoped, `account.tenant_id IS NOT NULL`). This was wrong: only personal accounts
(user/manager/staff, `account.tenant_id IS NULL`) can own cross-tenant identities.

**Problem:** `get_by_telegram_id` filtered `WHERE account.tenant_id = tenant_id`, which could
never find personal accounts. The per-tenant unique index `(telegram_user.id, tenant_id)` also
failed silently because `tenant_id` was NULL.

**Fix:** Global lookup (no `tenant_id` filter). Global unique index on `telegram_user.id` alone.
Login still scopes the issued connection string with the requested `tenant_id`.

**Rule:** Any identity or credential that must be valid across multiple tenants belongs on a
personal account, not a subscription account. Subscription accounts are inherently tenant-scoped.

---

### L-003: JWT Bearer vs connection string — different headers, never interchangeable (2026-04-20)

**Context:** Documentation for Telegram IdP used `Authorization: Bearer <connection_string>`, which
is wrong. Magic-link issues a JWT sent as `Authorization: Bearer <jwt>`. Telegram login issues a
connection string (`acc=...;tid=...;sig=...`) sent as `x-mycelium-connection-string: <string>`.

**Rule:** Never mix the two. A connection string sent as `Authorization: Bearer` fails JWT signature
validation and returns 401. The gateway checks `x-mycelium-connection-string` first, falls back to
Bearer only if absent — but the fallback is for JWT, not for connection strings.

**How to apply:** In documentation and client code, always use `Authorization: Bearer` for JWTs
(magic-link, email+password) and `x-mycelium-connection-string` for connection strings (Telegram
login, service tokens).

---

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
| 003 | RUSTSEC-2026-0104 rustls-webpki pin     | 2026-04-25 | `39151012`   | ✅ Done |

---

## Current Focus

**Standalone Mode — specified, ready for execution** (2026-07-06). Branch **`feat/standalone-mode`**
(gateway submodule) — NEVER work on `develop` for this feature (user instruction). Spec at
`.claude/specs/features/standalone-mode/` (spec.md + design.md + tasks.md). See AD-005 for rationale.
**Tracking epic: GitHub issue [#159](https://github.com/LepistaBioinformatics/mycelium/issues/159)**
— reformatted 2026-07-07 per user request to follow the org's feature-request draft pattern (see
project #5 item "Harden the release pipeline..." as the reference example): title `[FEAT] ...`,
sections "Is your feature request related to a problem?" / "Describe the solution you'd like" /
"Describe alternatives you've considered" / "Additional context", plus an **Acceptance checklist**
using requirement IDs (`SM-R1..R15`) phrased as WHEN/THEN clauses (only check a criterion when it is
genuinely, fully satisfied — not partially). The granular 27-task implementation checklist
(`SM-T1..T27`) is kept as a separate section below the acceptance checklist for day-to-day tracking.
Local copy of the current issue body: `/tmp/standalone-issue.md` (regenerate via `gh issue view 159
--json body -q .body` if lost). Keep both the task checkboxes AND the acceptance checklist in sync
as work lands — task checkbox ≠ requirement satisfied; check `SM-Rn` only when its WHEN/THEN holds
end-to-end.

| Phase | Status |
|---|---|
| Specify (spec.md — requirements + brief corrections C-1…C-7) | ✅ Done |
| Design (design.md — compile-time feature, SQLite/moka/email/secrets/docker) | ✅ Done |
| Tasks (tasks.md — 27 tasks, 8 groups; critical path G2 SQLite) | ✅ Done |
| Execute — SM-T1 (feature scheme + guards) | ✅ Done (committed `ce493b53`) |
| Execute — SM-T2 (CI builds both targets) | ✅ Done (committed `4f7f9354`) |
| Execute — SM-T3 (sqlite feature on `myc-diesel`; backend isolation verified) | ✅ Done (committed `4d36fc94`) |
| Execute — SM-T4/T5/T6 (sqlite scaffolding: pool+pragmas, schema+migrations, type helpers) | ✅ Done, verified |
| Execute — SM-T7 (account + account_tag SQLite repos, block 1/3) | ✅ Done (committed `5721c643`) |
| Execute — SM-T8 (tenant + tenant_tag SQLite repos, block 2/3) | ✅ Done (committed `34300a06`) |
| Execute — SM-T9 (user, block 3/3 — closes block 1) | ✅ Done (committed `2f7ad14a`) |
| Execute — SM-T10 (token; session_token confirmed dead code, skipped) | ✅ Done (committed `7a050e83`) |
| Execute — SM-T11 (guest_role + guest_user + guest_user_on_account) | ✅ Done (committed `7504af86`) |
| Execute — SM-T12 (message) | ✅ Done (committed `a8c47288`) |
| Execute — SM-T13 (webhook + webhook_execution) | ✅ Done (committed `4093da0e`) |
| Execute — SM-T14 (error_code) | ✅ Done (committed `978df130`) |
| Execute — SM-T15 (licensed_resource + ProfileFetching) | ✅ Done (committed `46904cfc`) |
| Execute — SM-T16 (encryption_key — closes G2 per-entity repos) | ✅ Done, verified |
| Architectural correction — extract SQLite adapter into `adapters/diesel_sqlite` (own crate, not nested in `mycelium-diesel`); `adapters/diesel` renamed `diesel_postgres` | ✅ Done (committed `c79c1f5d`) |
| Execute — SM-T17 (SqlAppModule sqlite wiring — barrier, closes G2) | ✅ Done (committed `f1f02d2b`) |
| Execute — SM-T18/T19 (moka cache: `adapters/moka_cache` crate, KVAppModule, per-key TTL) | ✅ Done (committed `a341089d`) — closes G3 |
| Execute — SM-T20/T21 (notifier `local-transport` feature: stub + file transport, SMTP precedence) | ✅ Done (committed `f325212e`) — closes G4 |
| Execute — SM-T22 (autogen secrets: keyring + encrypted-file fallback) | ✅ Done (committed `a7732274`) — closes G5 |
| Execute — SM-T23 (standalone config shape) | ✅ Done (committed `fcc08f08`) |
| Execute — SM-T24 (cfg-gated `initialize_modules` + `main()`, autogen secrets wired in) | ✅ Done, verified end-to-end (real boot test, 2 restarts) — closes G6 |
| Execute — SM-T25 (`Dockerfile.standalone`) | ✅ Done, verified with real `docker build`/`docker run`/`docker restart` |
| Execute — SM-T26 (zero-config E2E smoke) | ⚠️ Partial — boot+health covered; full JWT/route/proxy flow deferred (needs jwtSecret autogen) |
| Execute — SM-T27 (docs: `23-standalone-mode.md`, ROADMAP → IMPLEMENTED, i18n sync) | ✅ Done — closes G8 and the feature's initial implementation |

**SM-T1 result:** `ports/api` now has `default=["full"]` + no-op `standalone` marker + two
`compile_error!` guards. `cargo check` verified for default, `--no-default-features --features standalone`,
and the both-features failure. `fmt --check` clean. Not committed yet (awaiting user test/approval).

**SM-T7 result:** `adapters/diesel_sqlite/src/{models,repositories}/` account + account_tag trees
(details + landmines in tasks.md SM-T7 result block; path shown post architectural-correction — see
below). Key reusable lessons for T8+: enable `diesel/returning_clauses_for_sqlite_3_35`;
`account_type::jsonb @>` needs case-by-case full-equality vs. `json_extract` partial-match analysis,
don't assume uniform rewrite; `NaiveDateTime` Display/FromStr aren't symmetric (use the new
`naive_timestamp_{to,from}_text` pair); tables with `DEFAULT gen_random_uuid()` on postgres need the
app to supply the id explicitly on sqlite inserts. Integration test pattern established:
`test_support.rs` (temp DB + migrations) + per-entity lifecycle test. Not committed yet (awaiting user
test/approval).

**Architectural correction (post SM-T16):** the user flagged that SM-T3..T16 were built as a
`#[cfg(feature="sqlite")]`-gated `sqlite/` submodule nested inside `mycelium-diesel` — a structural
error, since every other adapter (`kv_db`, `mem_db`, `notifier`, `service`, `shared`) is its own
sibling crate under `adapters/`. Extracted the SQLite adapter into a new crate **`adapters/diesel_sqlite`**
(package `mycelium-diesel-sqlite`, lib `myc_diesel_sqlite`): moved all `sqlite/*` source + migrations,
flattened `crate::sqlite::` → `crate::` across 68 files, reverted `mycelium-diesel`
(`adapters/diesel`, later renamed to `adapters/diesel_postgres` to mirror `diesel_sqlite`'s naming —
package/lib names unchanged: `mycelium-diesel`/`myc_diesel`) to a plain unconditional Postgres-only
crate (no features, no cfg-gating), added
`mycelium-diesel-sqlite = { path = "adapters/diesel_sqlite" }` to the workspace
`[workspace.dependencies]` (picked up automatically by the `adapters/*` members glob). Verified:
`cargo build`/`test -p mycelium-diesel-sqlite` (22 tests green), `cargo build -p mycelium-diesel`
clean, `cargo build --workspace` clean, `cargo test --workspace --all` (0 failed across all crates),
`cargo fmt --all -- --check` clean (after `cargo fmt --all` reflowed the shortened `crate::` paths),
and `cargo build -p mycelium-api --no-default-features --features standalone` unaffected. **Standing
rule for all future adapter work in this codebase: new backend adapters are always separate sibling
crates under `adapters/`, never a feature-gated submodule nested inside an existing adapter crate**
(applies to SM-T18's future cache crate too). Not committed yet (awaiting user test/approval).

**SM-T8 result:** tenant + tenant_tag SQLite repos, reusing `account/shared.rs::created_at_from_text`
and `json_array_to_text`/`from_text`. Found and fixed a missing `joinable!(owner_on_tenant -> user
(owner_id))` in sqlite/schema.rs (present in postgres, missed in SM-T5). Discovered (not fixed, see
tasks.md SM-T8 result) a dormant postgres write-path bug in tenant status serialization, and two
JSON filters in `filter_tenants_as_manager` needing SQLite-specific (non-byte-identical but
practically-equivalent) rewrites. User directive: "pode seguir até terminar a implementação da
feature" — continue through all remaining tasks (SM-T9..T27) without stopping for per-task
confirmation. Following the pattern already established and accepted earlier in this session
(commit after each task once fully gate-checked: fmt + full-mode build/test + sqlite tests), continue
committing at each verified step rather than batching everything into one giant uncommitted diff —
easier to review/bisect. Never skip the build/test gate regardless.

**Block 1 (account + tenant + user) is done** — 17 repo impls, ~2,900 lines, 3 end-to-end lifecycle
tests (see tasks.md SM-T9 result for the full summary and reusable patterns established).

**SM-T10 result:** token repos done (registration/fetching/invalidation/deletion). Confirmed via grep
that `session_token` (core traits `SessionTokenRegistration`/`Fetching`/`Deletion`) has **zero
implementations anywhere in the codebase** — not postgres, not wired into any shaku module, not
consumed by any use-case. Vestigial dead trait code; removed from the standalone scope, nothing to
port. First genuine JSON1 *mutation*: `jsonb_set(meta,'{token}','null'::jsonb)` → `json_set(meta,
'$.token', json('null'))` (magic-link phase-1 consumption) — verified end-to-end via the new
two-phase-consumption test. Also closed a latent SQL-injection gap: postgres's token_invalidation/
token_deletion build raw SQL via inconsistent string interpolation (2 of 4 methods escape quotes, 2
don't); the SQLite port binds all parameters uniformly instead.

**SM-T11 result:** guest_role + guest_user + guest_user_on_account done. Key lesson (see tasks.md
SM-T11 result for full detail): **timestamp convention is per-table, not per-feature** — `guest_user`'s
own table uses genuine `DateTime<Local>` round-trip (needs `timestamp_to_text`/`from_text`), while
`guest_role`/`guest_role_children`/`guest_user_on_account` use the naive-reinterpretation convention
(needs `naive_timestamp_to_text`/`from_text`) — always check the postgres model field type
(`NaiveDateTime` vs `DateTime<Local>`) before picking a helper, never assume uniformity across a
whole feature. Also: Diesel's `.nulls_last()` is postgres-only — use `ORDER BY (col IS NULL), col
DESC` instead. Confirmed `guest_role_children`/`guest_user_on_account` are two more tables relying on
postgres server-side defaults (`created`) with no SQLite equivalent — explicit `created` on every
insert into these tables (growing list since SM-T7's `manager_account_on_tenant` finding: **always
check for server-side defaults before writing a SQLite insert**).

**G2 per-entity repos are all done** (SM-T7..T16 — 14 entity groups, ~44 postgres repo impls now
have SQLite equivalents; `session_token` excluded as dead code).

**SM-T17 result — closes G2:** `SqlAppModule` shaku module added to
`adapters/diesel_sqlite/src/repositories/mod.rs`, mirroring postgres's `SqlAppModule` 1:1 (44
components: `DieselSqliteDbPoolProvider` + 43 repos across all 10 entity groups). Own crate, no
cfg-gating needed. Compiled clean first try. 22/22 sqlite tests + full workspace build/test (0
failed) + fmt clean + standalone binary unaffected. Committed `f1f02d2b`.

**SM-T18/T19 result — closes G3:** New sibling crate `adapters/moka_cache`
(`mycelium-moka-cache`/`myc_moka_cache`) — corrected from the design doc's original "one `adapters/
cache` crate with `redis`/`moka` features" sketch, per the same adapter-crate-separation rule applied
to G2; `adapters/kv_db` (Redis) is untouched. `MokaCacheProvider` + `MokaCacheProviderImpl` wrap
`Arc<Cache<String, (String, Duration)>>` built with a `PerKeyExpiry: moka::Expiry` impl that returns
each entry's own stored `Duration` from `expire_after_create` — gives true per-key TTL (SM-R5), not
moka's uniform `time_to_live`. `KVArtifactRead`/`WriteRepository` mirror `kv_db`'s shape exactly
(`#[shaku(inject)] provider`), with `pub(crate)` fields so tests construct repos directly (same
convention as `diesel_sqlite`'s `pub(crate) pool`). `KVAppModule` shaku module named identically to
`kv_db`'s (never linked together — compile-time selection in SM-T24, same as the two `SqlAppModule`s).
3 tests: round trip, `NotFound` on miss, and a **real-time** TTL-expiry test (ttl=1s, sleep 1.2s,
assert evicted) — not just a compile-time claim that `PerKeyExpiry` works. `moka 0.12.15` resolved
and compiled clean from crates.io on the first attempt. Verified: 3/3 moka tests, full workspace
build/test (0 failed), fmt clean, standalone binary unaffected, `kv_db` untouched. Committed `a341089d`.

**SM-T20/T21 result — closes G4:** `local-transport` Cargo feature added to the *existing*
`mycelium-notifier` crate (not a new sibling crate — this is a feature on one crate, one port
(`RemoteMessageWrite`), not a second storage backend nested in another adapter, so the
adapter-crate-separation rule doesn't apply here). Correction versus the design doc: lettre 0.11 has
no `stub-transport` feature — `StubTransport` is always compiled; only `file-transport` needed
enabling. Extracted `repositories/shared.rs::build_lettre_message` from the existing
`remote_message_sending.rs` so the new code doesn't duplicate the from/to/builder logic. New
`local_transport_sending.rs`: `LocalTransportKind` enum wrapping the real `SmtpTransport`/
`FileTransport`/`StubTransport`, `select_local_transport(smtp, file_dir)` — a pure function
implementing the SMTP→file→stub precedence (SM-R8) — and one `LocalTransportMessageSendingRepository`
`RemoteMessageWrite` impl covering all three transports, so SM-T24's DI wiring only ever swaps a
single component type, matching the two-`SqlAppModule`/two-`KVAppModule` pattern. Wired into a new,
separate `LocalNotifierAppModule` (cfg-gated) rather than into the existing `NotifierAppModule` —
shaku doesn't support two components for the same interface in one module, and this keeps full mode's
module completely untouched (SM-R14). Tests: 3 precedence unit tests, one capturing real `tracing`
output (custom `MakeWriter`) to prove the stub path logs subject/recipient/body, one sending through a
real `FileTransport` and reading the `.eml` back to assert it's parseable. Verified: 8/8 notifier
tests under `--features local-transport`, 3/3 unchanged under default (byte-identical), full
`cargo build --workspace` + `cargo test --workspace --all` (0 failed) + `fmt` clean, standalone binary
unaffected. Committed `f325212e`.

**SM-T22 result — closes G5:** New `standalone-secrets` feature on `mycelium-config`
(`dep:keyring`/`dep:ring`/`dep:uuid`, all optional). `resolve_or_generate_standalone_secret(service,
secrets_dir, name)`: keyring → encrypted file (0600, AES-256-GCM via the same `ring` crate `core`
already uses for envelope encryption, no new crypto dep) → generate-and-persist, called only when no
explicit secret was configured (upstream `SecretResolver::Env`/`Value` path unchanged). **Real bug
caught during testing, not just a hypothetical**: this sandbox's D-Bus secret-service backend returned
success from `set_password` without durably persisting the value — a naive implementation would have
silently regenerated `token_secret` on every boot, which AD-004 says is catastrophic (rotates the KEK,
invalidates every persisted connection string). Fixed by verifying every keyring write with an
immediate read-back before trusting it; falls through to the file otherwise. 8 tests, including the
full first-boot-generate/second-boot-reuse lifecycle re-run 5x to confirm the fix isn't flaky.
Verified: 8/8 config tests, full workspace build/test (0 failed), fmt clean, standalone binary
unaffected. Committed `a7732274`.

**Refactor before SM-T23/24 — `active_backend_modules` indirection (committed `6ecfeeec`):** the
advisor flagged SM-T24 as the real risk point — 46 files reference `SqlAppModule` directly, 3
reference `KVAppModule`, all via `req.app_data::<web::Data<X>>()`/`.resolve_ref()` (pure shaku
resolution, no construction logic). Introduced
`ports/api/src/models/active_backend_modules.rs` as a single cfg-gated re-export point and redirected
every one of those files to import from it instead of the concrete adapter crate. `NotifierAppModule`
and `SharedAppModule` are referenced ONLY from `main.rs` (confirmed by grep), so they need no such
indirection — `main.rs` can cfg-branch its own imports directly. `MemDbAppModule` (13 files) is
backend-agnostic and untouched in both modes. This step alone is a pure refactor (postgres branch
only, so far) — verified full-mode byte-identical (all tests, same counts, before/after).

**SM-T23 result (committed pending):** see tasks.md SM-T23 result for full detail. `ConfigHandler`
cfg-gated (`diesel`/`smtp`/`queue`/`redis`/`vault` under `full`, new `sqlite: SqliteConfig`
under `standalone`); new `SqliteConfig` type in `adapters/diesel_sqlite/src/config.rs`; shipped
`settings/config.standalone.example.toml` (no smtp/redis/queue/vault, `[sqlite] path`, `[auth]
internal = "disabled"` for now, placeholder token/hmac secrets documented as boot-time-overridden).
Also registered `mycelium-diesel-sqlite`/`mycelium-moka-cache` as optional deps on `ports/api` gated
by `standalone`, and wired `standalone` to enable `mycelium-notifier/local-transport` +
`mycelium-config/standalone-secrets`.

**SM-T24 result — closes G6, standalone genuinely boots:** split `initialize_modules` into two
`#[cfg]`-gated bodies sharing a new `build_mem_db_module` helper (the service/callback registry is
backend-agnostic). Standalone: `provision_database(&sqlite_path)` (new `diesel_sqlite::migration`
function: creates parent dir, establishes a connection, runs embedded migrations) → `SqlAppModule`;
`KVAppModule` via `MokaCacheProviderImplParameters { cache: MokaCacheProviderImpl::new().cache }`
(required bumping that field from `pub(crate)` to `pub` for cross-crate construction, same for
notifier's `LocalTransportKind` field); `LocalNotifierAppModule` via `select_local_transport(None,
None)` (standalone has no SMTP config surface yet, so always resolves to stub — documented
limitation, doesn't block the zero-external-services boot requirement). Dropped
`SharedAppModule`/`shared_module` entirely for standalone (confirmed nothing outside `main.rs` ever
resolves it). `main()`: vault init + the `initialize_modules()` destructuring (5-tuple vs 4-tuple) are
cfg-gated; `shared_module`'s `app_data` registration became its own cfg-gated `let base_app =
base_app.app_data(...)` rebind (mid-chain `#[cfg]` on a single `.app_data()` call isn't legal Rust).
Token/HMAC secrets resolved right after config load via `resolve_or_generate_standalone_secret`
(secrets dir = sqlite path's parent + `.secrets`) and injected via the existing
`with_token_secret_override` plus a new, small, additive `with_hmac_secret_override` on
`core::AccountLifeCycle` (mirrors the existing method; core now 203/203 green with the new test).

**Two real bugs found via actual boot testing, not just compilation:**
1. TOML table-ordering bug in `config.standalone.example.toml` — `tls`/`routes` placed after
   `[api.logging]` got silently attributed to that sub-table instead of `[api]` (TOML semantics), so
   deserialization failed with "missing field `tls`". Fixed by reordering + a comment explaining why.
2. Genuine pre-existing coupling bug in `mycelium-notifier`: `QueueConfig`/`SmtpConfig` shared one
   private `TmpConfig { smtp, queue }`, so loading `[queue]` alone (now unconditional on
   `ConfigHandler` since it's backend-agnostic dispatcher-polling config, not Redis-specific) always
   required `[smtp]` too. Fixed by splitting into two module-local `TmpConfig` structs and deleting
   the shared file — no public API change, full-mode `[smtp]` still required, 3/3 notifier tests green.

**Verified end-to-end:** ran the real `myc-api` binary (`--no-default-features --features standalone`)
against a minimal TOML + empty temp dir. First boot: auto-provisioned SQLite (18 tables), served
`GET /health` successfully. **Second boot** — a genuinely separate OS process against the same
paths — also booted clean and served, confirming persistence holds across a real restart, not just
within one test process. Full workspace build/test (0 failed everywhere), fmt clean, both
`mycelium-api` builds (default + standalone) clean with zero warnings. Not committed yet.

**SM-T25 result — Dockerfile.standalone, verified with a real Docker daemon (available in this
environment):** multi-stage build, builder from local source (not `cargo install` — the published
crate has no `standalone` feature), runtime `debian:bookworm-slim` + `ca-certificates` only, `/data`
volume, config baked in as the zero-args default. **Real bug caught by an actual build+run**:
`rust:latest` (builder) produces a binary linked against a newer glibc than `debian:bookworm-slim`
(runtime) ships — `GLIBC_2.38'/2.39' not found` at container startup. Fixed by pinning the builder to
`rust:bookworm`. After the fix: `docker build` succeeded (52MB image), `docker run` with a mounted
volume booted clean, auto-provisioned the SQLite DB, and served `/health`. Because a container
genuinely has no keyring backend (unlike this session's bare-metal test earlier), the secret resolver
exercised the **encrypted-file fallback for the first time end-to-end** — `token_secret.secret`/
`hmac_secret.secret` appeared under `/data/.secrets` with correct `0600` perms, exactly OC-2's
anticipated primary path. A full `docker restart` (genuine container restart) still served `/health`
afterward. Test image/container removed post-verification.

**SM-T26 status — partially covered, not fully claimed:** the boot+health portion is solidly covered
by SM-T24 (2 bare-metal boots) + SM-T25 (Docker boot + restart) above. The full scenario (issue a JWT
via magic-link, add a route, proxy a request) needs internal auth, which the shipped config leaves
disabled since `jwtSecret` isn't yet wired into the autogen-secrets flow (`token_secret`/`hmac_secret`
are; `jwtSecret` would need the same `with_*_override` treatment). Documented as a fast-follow rather
than silently claimed done.

**SM-T27 result — closes G8 and the feature:** new `docs/book/src/23-standalone-mode.md` (added to
`SUMMARY.md`) covers what changes vs. full mode, build/run/Docker instructions, the secret
resolution order with a backup callout, and all six L-1..L-6 limitations framed as trade-offs, not a
degraded full mode. `ROADMAP.md`'s entry moved SPECIFIED → IMPLEMENTED with a "Known gap" callout for
the deferred jwtSecret/full-E2E work. i18n catalog refreshed per the `docs-i18n-sync` rule — found
that the rule's literal `mdbook`/`MDBOOK_OUTPUT` command doesn't write to `po/messages.pot` in this
mdbook-i18n-helpers version (writes to `<build-dir>/messages.pot` instead); worked around by copying
that file into place before `msgmerge`, achieving the rule's actual intent (40 new strings picked up
in `pt-BR.po`). Verified: full workspace build/test (0 failed), fmt clean, both `mycelium-api` builds
clean, both English and `pt-BR` `mdbook build` succeed.

**Standalone Mode feature status: G1 through G8 all done.** The zero-external-dependencies gateway
genuinely boots, auto-provisions its database, and serves requests — verified across bare-metal
restarts and a real Docker build/run/restart cycle, not just compiled.

**Post-G8 follow-up (2026-07-07, user-driven):** the user asked what was actually missing for JWT to
work, prompting a closer look that found and fixed real gaps rather than just answering the question:
- `[auth.internal.define]` is the correct TOML shape to enable internal auth — `internal = "enabled"`
  alone does not work (`OptionalConfig::Enabled` is a newtype, not a bare string). Discovered the
  repo's own `settings/config.for-docker.toml` has this exact latent bug (pre-existing, unrelated to
  standalone work, not fixed — out of scope), confirmed by an empirical parse test.
- Wired `jwtSecret` into the same autogen-secrets flow as `tokenSecret`/HMAC (opt-in via
  `[auth.internal.define]`; verified end-to-end with a real magic-link request through a running
  standalone instance — health check, `POST magic-link/request`, stub transport logging the link).
- **Found and fixed a real correctness bug while verifying the docs**: `tokenSecret`/`hmacSecrets`/
  `jwtSecret` were always overridden unconditionally in standalone, ignoring any operator-supplied
  `{ env = "..." }` value — contradicting SM-R9's own resolution order. Fixed with two small
  read-only accessors on `core::AccountLifeCycle` (`token_secret_resolver`,
  `primary_hmac_secret_resolver`) so `ports/api` can distinguish an explicit `Env` resolver from the
  shipped placeholder literal before falling back to keyring/file/generate.
- Extended `Email::from_string`'s regex to accept `localhost` as a domain (both build modes), per
  user request, so `noreplyEmail`/`supportEmail` don't need a fake real-looking domain.
- Committed `5e6a74df`. Full workspace build/test (0 failed, 206 core tests), fmt clean, both
  `mycelium-api` builds clean, real boot test with the full magic-link flow working, both English and
  `pt-BR` `mdbook build`.

**G9 — post-issue-audit follow-ups (2026-07-08/09):** auditing GitHub issue #159's acceptance
checklist against the actual code found the checklist itself had drifted (several already-satisfied
requirements left unchecked — corrected directly on the issue), plus three genuine gaps closed here:
- **SM-T28** — corrected `SM-R10`'s wording: `[queue]` is required-but-backend-agnostic in
  standalone (not "optional/irrelevant" like `[redis]`/`[smtp]`/`[vault]`, which are genuinely
  absent). Doc-only, `spec.md` updated.
- **SM-T29 — closes `SM-R8`:** real SMTP is now opt-in in standalone via an optional `[smtp]`
  section (`OptionalConfig<SmtpConfig>`, same pattern as `[vault]`). Extracted
  `SmtpConfig::build_transport()` so full mode's `NotifierClientImpl` and standalone's new wiring
  share one construction path instead of duplicating it. Absent `[smtp]` → byte-identical stub
  fallback as before.
- **SM-T30 — closes `SM-R13`, completes `SM-T26`:** new `scripts/standalone-e2e-smoke.sh` drives the
  full zero-config flow end-to-end and repeatably (ran twice, both green): health → magic-link
  request → display → verify (JWT issued) → downstream route (config-driven, not REST — confirmed
  by reading `ApiConfig::deserialize_services`) → proxy through it, reusing the repo's own
  `test/downstream_service` as the target. Two real integration issues found only by running it live:
  the email dispatcher's polling means the stub-transport log line appears asynchronously (script
  polls for it); Tera auto-escapes the rendered email HTML (`/`→`&#x2F;`, `&`→`&amp;` inside
  `href="..."`), so extraction decodes the escaped form.

Verified: full workspace gate (`cargo fmt --all -- --check`, `cargo build --workspace` — clean,
`cargo test --workspace --all` — 206 core tests + all other crates, 0 failed) and
`cargo build -p mycelium-api --no-default-features --features standalone` all green after G9.
Docs (`23-standalone-mode.md` + i18n) and GitHub issue #159 updated to match. **Not committed yet
(awaiting user test/approval per commit-validation rule).**

**Next action:** none pending — G9 closes the last three open items from the issue audit
(`SM-R8`/`SM-R10`/`SM-R13`). Await user direction (test + approve for commit, or further review).

**Needs / reminders to resume:**
- Work only on `feat/standalone-mode`; keep full mode byte-identical after every task (SM-R14).
- G2 (SQLite backend) is the dominant cost — ~44 repo impls, two portability axes (types + JSON1
  pg-isms per OC-4). Land per-entity groups incrementally, full mode green throughout.
- Secrets must be generated once and persisted (regenerating rotates the KEK — see AD-004); keyring
  usually absent on targets → encrypted-file fallback is primary (OC-2).
- Commit-validation rule stands: no code commit until the user tests and approves.

---

**HMAC Key Rotation — shipped** (2026-04-25). Spec `.claude/specs/features/hmac-key-rotation/`
at the monorepo root. PR #151 merged into `develop` (gateway HEAD `aae89c96`); monorepo
pointer `e955f50` on `main`.

| Etapa | Scope | Status | Commit |
|---|---|---|---|
| E1 | Decouple `hmac_secret` from `token_secret` (additive, fallback warn) | ✅ Done | `d4e21e94` |
| E2 | `myc-cli rotate-kek` (re-wrap DEKs, no token invalidation) | ✅ Done | `2a6ebfbf` |
| E3 | KVR versioned signing — **BREAKING** (no fallback) | ✅ Done | `b2d4685f` |
| Hotfix | RUSTSEC-2026-0104 pin (`rustls-webpki >= 0.103.13`) | ✅ Done | `39151012` |

**Open follow-ups:**
- **E2-T4** (Postgres integration test for `rotate-kek`) — deferred to roadmap M2 "Database
  Integration Tests" (no Postgres test scaffold in repo yet). Migrator covered by helper-layer
  unit tests.
- **Operator deployment of Etapa 3** — separate coordination required. Etapa 3 invalidates
  every connection string issued before deploy; plan the rollout during a re-auth-tolerant window.

---

**Telegram IdP — implementation complete, conceptual fix applied** — branch `feat/messaging-platform-idp/telegram`.

| Task | Status | Commit |
|---|---|---|
| T13 — TelegramUser DTO + AccountMeta key | ✅ Done | `12f80f53` |
| T14 — TenantMeta keys + TelegramConfig trait | ✅ Done | `12f80f53` |
| T15 — POST /auth/telegram/link | ✅ Done | `12f80f53` |
| T16 — DELETE /auth/telegram/link | ✅ Done | `12f80f53` |
| T17 — POST /auth/telegram/login/{tenant_id} | ✅ Done | `12f80f53` |
| T18 — POST /auth/telegram/webhook/{tenant_id} | ✅ Done | `12f80f53` |
| Encrypted config — POST /tenant-owner/telegram/config | ✅ Done | `12f80f53` |
| Fix: personal-account model (OQ-2b superseded) | ✅ Done | `ef8a707e` |
| T19 — Mode B routing (identity_source on Route) | ✅ Done | `735ddaf` |
| Post-T19 — BodyIdpResolver trait + TelegramIdpResolver | ✅ Done | `afa5b915` |
| Post-T19 — Screaming-architecture rule (`.claude/rules/`) | ✅ Done | `afa5b915` |
| Post-T19 — `IdentitySource` moved to `identity_source.rs` | ✅ Done | `afa5b915` |
| Post-T19 — `prepare_body_idp_context` pipeline module | ✅ Done | `afa5b915` |
| Post-T19 — `06-downstream-apis.md` docs (`allowedSources`, `identitySource`, CORS clarification) | ✅ Done | `c2dd1251` |
| Docs — `10-alternative-idps.md` (admin + user journeys, real-world examples) | ✅ Done | `3f373249` |
| Docs — tenant config scope clarification (what works without config) | ✅ Done | `912fbfd2` |
| Docs — JWT vs connection-string disambiguation | ✅ Done | `64c8d866` |

**Key decisions:**
- Secrets stored as AES-256-GCM ciphertext (`base64(nonce‖ct‖tag)`) — not plain text, not Vault ref
- Key derived from `AccountLifeCycle::token_secret` (same pattern as `HttpSecret`)
- `TelegramBotTokenRef` / `TelegramWebhookSecretRef` renamed to `TelegramBotToken` / `TelegramWebhookSecret`
- `TelegramConfigSvcRepo::from_tenant_meta` is now `async`, decrypts eagerly
- **OQ-2b superseded (2026-04-19):** Telegram identity links to personal accounts (user/manager/staff), not subscription accounts. Personal accounts have no `tenant_id` column. `get_by_telegram_id` is a global lookup. The unique DB index is now global (`idx_account_meta_telegram_user_id_global`). Login still scopes the connection string to the requested tenant.
- `AllowedAccounts(vec![])` bug fixed in `link_telegram_identity` and `unlink_telegram_identity` — was generating `WHERE id IN ()` (always false)

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
- [ ] `TelegramConfig` trait está em `core/domain/entities` mas nenhum use case do core a usa — apenas o port handler a consome diretamente via shaku. Isso viola o espírito da arquitetura hexagonal (traits no core deveriam ser portas para use cases, não para ports). Opções: mover o trait para `adapters/service` como tipo concreto, ou criar um use case de "resolve config" que o port chame. Capturado durante: Telegram IdP (2026-04-19)
- [ ] Email address validation in the DTO layer (not just at send time) — Captured during: fix-notifier-panics
- [ ] Hot-reloading Tera templates (ops/config concern) — Captured during: fix-notifier-panics
- [ ] **CI job: assert `up.sql` alone and `up.sql` + `sql/migrations/*` produce identical `pg_dump --schema-only` output.** `CONVENTIONS.md` now *states* that every migration must be folded into `up.sql`; only CI can *enforce* it. Without this, the two paths silently diverge the first time someone adds a migration and forgets the fold — which is exactly how `instance_settings`/`resource_audit_log` lost their grants. A working `verify.sh` was written during the feature and is the basis for the job. — Captured during: single-file-postgres-install (AD-010)
- [ ] **`embed_migrations!` on the Postgres adapter**, so Postgres self-installs and self-migrates like SQLite does. Would eliminate operator-applied SQL entirely. Blocker to think through first: in a multi-pod Kubernetes deployment every replica would race to migrate at boot — needs an advisory-lock or init-container/Job strategy before it's safe. — Captured during: single-file-postgres-install (AD-010, D-06)

---

## Release Automation (2026-04-26)

### Completed

| Item | Status | Detail |
|---|---|---|
| `release-prerelease.yml` | ✅ Done | `workflow_dispatch` on `develop` — bumps `beta` / `rc` via `cargo release` |
| `release-stable.yml` | ✅ Done | `workflow_dispatch` on `main` — bumps `patch` / `minor` / `major` via `cargo release` |
| `docker-release.yml` | ✅ Done | Triggers on tag push + `workflow_dispatch`; builds from `Dockerfile.dev`; pushes to `ghcr.io/LepistaBioinformatics/mycelium` |
| First pre-release image | ✅ Done | `8.3.1-rc.2` built and pushed to GHCR manually (tag push webhook missed) |

### crates.io publish — ✅ Complete (2026-04-26)

All 13 workspace crates confirmed to exist on crates.io (no name conflicts). Workflows updated:
- `release-prerelease.yml` and `release-stable.yml`: removed `--no-publish` from execute step, added `CARGO_REGISTRY_TOKEN` env at job level
- `docker-release.yml`: switched `file: Dockerfile.dev` → `file: Dockerfile`, added `build-args: VERSION=${{ steps.tag.outputs.version }}`

**Remaining manual step:** Add `CARGO_REGISTRY_TOKEN` secret to the GitHub repository settings before triggering the next release.


### Branch semantics

| Branch | Allowed release types | Notes |
|---|---|---|
| `develop` | `beta`, `rc` | Pre-release ladder |
| `main` | `patch`, `minor`, `major` | Stable only; `rc → stable` graduation runs here after PR merge |

### Image tagging strategy (GHCR)

| Tag format | Images produced |
|---|---|
| `8.3.2` (stable) | `:8.3.2`, `:latest` |
| `8.3.2-rc.1` | `:8.3.2-rc.1`, `:rc` |
| `8.3.2-beta.1` | `:8.3.2-beta.1`, `:beta` |

### Tag/version naming convention (REL-14/15, 2026-07-11)

Git tags and image tags use **no `v` prefix** (`8.3.2`, not `v8.3.2`) — this is what
`release.toml`'s `tag-name = "{{version}}"` and every current image tag already produce; this
section just makes the convention explicit. Historical `v*` tags/Releases (pre-`8.3.0`) are left
as-is, never renamed. See `docs/book/src/08-release-process.md` for the full release process.

---

## Todos

- **Release pipeline hardening** — spec/design/tasks at `features/release-pipeline-hardening/` (2026-07-06, design 2026-07-11). Workflow YAML implemented: GitHub Release automation (`docker-release.yml`'s new `github-release` job), OIDC Trusted Publishing wiring (`rust-lang/crates-io-auth-action`, `CARGO_REGISTRY_TOKEN` kept as fallback), the `workflow_dispatch` ref bug (`docker-release.yml` checkout had no `ref:` at all), build provenance attestation + cosign keyless signing, third-party actions pinned to commit SHA, and a `custom_version` input on `release-prerelease.yml` for the 9.0.0-rc.1 major-bump edge case cargo-release's LEVEL keywords can't express. **Still open (external, can't be done from code):** configure crates.io Trusted Publishing per-crate (15 crates), create the GitHub `release` Environment with required reviewers, remove `CARGO_REGISTRY_TOKEN` only after an RC proves OIDC works end-to-end. See `tasks.md` for the full 🤖/🧑 split.
- Retroactively create GitHub Releases for `8.3.0`+ tags and publish/discard the stale `8.3.1-rc.2` Draft.

---

## Preferences

**Model Guidance Shown:** never
