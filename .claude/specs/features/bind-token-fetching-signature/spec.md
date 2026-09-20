# Fix: bind the connection-string signature instead of interpolating it into raw SQL

**Issue:** [LepistaBioinformatics/mycelium#191](https://github.com/LepistaBioinformatics/mycelium/issues/191)
**Status:** Implemented — gates green (464 tests, 0 failures), awaiting user UAT before commit
**Scope:** Medium (1 file edited)
**Severity:** Low today, structural — defense-in-depth
**Branch:** `fix/bind-token-fetching-signature` (from `develop`)

---

## Problem

`adapters/diesel_postgres/src/repositories/token/token_fetching.rs` built the
`get_connection_string` query with `format!` and handed the string to `diesel::sql_query`:

```rust
WHERE elem->>'sig' = '{}'
```

`signature` is the `sig` bean of the client-supplied `x-mycelium-connection-string` header.
`account_id` is a `Uuid` and is safe by type.

**Why it was not exploitable:** `ports/api/src/middleware/fetch_connection_string_from_request.rs:86-96`
calls `scope.verify_signature(&AccountLifeCycle)` before any store access, so reaching the query
with an arbitrary `sig` requires forging a valid HMAC — and a valid HMAC is hex, not SQL. The
*ordering* is what made it safe, not the query.

**Why it still needed fixing:** that ordering lives in another layer, and nothing in the repository
asserted it. Any refactor moving verification, adding a second caller, or reusing the helper for an
unverified lookup would have turned it into a live injection.

---

## Requirements

- **R-1** — the `sig` and `aid` predicates of `get_connection_string` are bound parameters, not
  interpolated text.
- **R-2** — the honest lookup keeps its current behavior (a token whose scope carries a matching
  `aid` and `sig` is still found).
- **R-3** — no new dependency, no new abstraction. The sibling SQLite adapter already solved this;
  mirror it rather than inventing a second pattern.

---

## Design

The SQLite twin (`adapters/diesel_sqlite/src/repositories/token/token_fetching.rs:88-104`) already
binds both values. The Postgres adapter is brought to parity: `$1`/`$2` positional placeholders,
`.bind::<Text, _>` in call order, surrounding single quotes dropped (`elem->>'sig' = $2`, not
`'$2'`).

`account_id` is bound as `Text` because `elem->>'aid'` yields `text`; `Uuid::to_string()` produces
the same literal the old `format!` emitted.

### Deliberately left alone

`list_connection_strings_by_account_id` in the same file interpolates `account_id::text = '"{}"'`.
The value is a `Uuid` — safe by construction, `Display` cannot emit a quote — and those embedded
double quotes are the JSONB text form, so a naive bind would change the comparison semantics.
Out of scope for this issue.

---

## Verification

`cargo fmt --all -- --check` clean, `cargo build --workspace` clean,
`cargo test --workspace --all` → **464 passed, 0 failed**.

`adapters/diesel_postgres` has no DB integration harness, so the change is not covered by an
automated test. Behavior was verified instead against a disposable `postgres:16-alpine` using the
same table shape and scope JSON, comparing the two query forms with an injection payload as `sig`:

| Query form | `sig` value | Rows |
|---|---|---|
| parameterized (new) | `deadbeef` (honest) | 1 — R-2 holds |
| parameterized (new) | `x' OR '1'='1` | **0** — payload treated as data |
| interpolated (old) | `x' OR '1'='1` | **1** — token returned despite a wrong signature |

Row 3 is the concrete failure the ordering invariant was hiding.

---

## Follow-up found while fixing this — not in this PR

Mapping the other `format!` + `sql_query` sites in `adapters/diesel_postgres` surfaced a
**materially more serious, non-hypothetical** instance of the same class:

`adapters/diesel_postgres/src/repositories/licensed_resources/licensed_resources_fetching.rs:78`
interpolates `role.name` raw into `gr_slug = '{}'`. Traced backwards:

```
x-mycelium-role request header (client-supplied, free-form JSON array of strings, unvalidated)
  → MyceliumProfileData::from_request  (ports/api/src/dtos/mycelium_profile_data.rs:123-152)
  → recovery_profile_from_storage_engines → fetch_profile_from_email
  → list_licensed_resources(roles)        → gr_slug = '<raw>'
```

`insert_role_header` overwrites that header with server-controlled `SystemActor` strings — but only
on the scopes that wrap it (`staffs`, `managers`, `rpc`). The `audit` scope
(`ports/api/src/main.rs:742`) does **not**, and `rest/audit/resource_audit_trail_endpoints.rs:108`
takes `MyceliumProfileData` directly — confirmed reachable. Other unwrapped scopes exist
(`/auth/telegram`, `/mcp`) but were not traced.

`strip_inbound_mycelium_headers` does not help here: its only non-test call site is
`router/initialize_downstream_request.rs:218`, i.e. the proxy path, not the native API extractors.
There is no HMAC gate on this path either, and a novel role string misses the profile cache
(`hash_profile_request` keys on the role list, `recovery_profile_from_storage_engines.rs:117-135`),
so the request reaches Postgres.

The same file also interpolates `email.email()`, and `token_invalidation.rs` interpolates
`meta.email.username` / `meta.email.domain` raw in two of its four functions (the two magic-link
functions already escape with `.replace('\'', "''")`). Those are currently constrained by the
`Email` regex in `core/src/domain/dtos/email.rs:18-20`, which admits no quote — the same
"safe by an invariant elsewhere" shape as #191.

The SQLite sibling already carries a `sql_quote()` helper with a comment explaining why a static
bind chain cannot cover a variable-arity filter list
(`adapters/diesel_sqlite/.../licensed_resources_fetching.rs:39-45`). The Postgres sibling never got
it. Fix is to mirror that helper.

Filed separately — different severity, different blast radius, should not ride on a
defense-in-depth PR.
