# Fix: escape free-form strings in the Postgres licensed-resources and token-invalidation queries

**Advisory:** GHSA-g35x-pxpx-vrqc (private draft)
**Status:** Implemented — gates green (467 tests, 0 failures), awaiting user UAT before commit
**Scope:** Medium (2 files edited + 3 unit tests)
**Severity:** High — authenticated SQL injection reachable through the public API
**Branch:** `fix/bind-licensed-resources-role-slug` (from `develop`)
**Sibling:** `bind-token-fetching-signature` (issue #191) — same defect class, that one had a gate in
front of it, this one does not

---

## Problem

`adapters/diesel_postgres/src/repositories/licensed_resources/licensed_resources_fetching.rs:78`
interpolated a client-controlled role name straight into the query:

```rust
format!("{}(gr_slug = '{}' AND gr_perm >= {}) OR ", acc, role.name, ...)
```

### The path

```
x-mycelium-role  (request header, free-form JSON array of strings, no validation)
  → MyceliumProfileData::from_request       ports/api/src/dtos/mycelium_profile_data.rs:123-152
  → recovery_profile_from_storage_engines   (cache miss → datastore)
  → fetch_profile_from_email                core/.../service/profile/fetch_profile_from_email.rs:48
  → list_licensed_resources(roles)          → gr_slug = '<attacker string>'
```

Three things that could have closed this path, all checked:

| Candidate mitigation | Verdict |
|---|---|
| `insert_role_header` overwrites the header with `SystemActor` strings | Only on the scopes that wrap it: `staffs`, `managers`, `rpc`. The `audit` scope (`ports/api/src/main.rs:742`) does not, and `rest/audit/resource_audit_trail_endpoints.rs:108` takes `MyceliumProfileData` directly. |
| `strip_inbound_mycelium_headers` removes client `x-mycelium-*` | Only non-test call site is `router/initialize_downstream_request.rs:218` — the proxy path, not the native API extractors. |
| Profile cache short-circuits before the DB | `hash_profile_request` keys on the role list (`recovery_profile_from_storage_engines.rs:117-135`), so a novel role string is always a miss. |

No HMAC anywhere on this path. A valid session is the only requirement. Other unwrapped scopes exist
(`/auth/telegram`, `/mcp`) and were not traced.

The same function also interpolated `email.email()` raw, and
`token_invalidation.rs:52-64` / `:151-163` interpolated `meta.email.username` / `meta.email.domain`
raw — currently contained by the `Email` regex (`core/src/domain/dtos/email.rs:18-20`), which admits
no quote, but contained by an invariant in another layer, exactly like #191. The two magic-link
functions in that same file already escaped; these two were missed.

---

## Requirements

- **R-1** — no free-form string reaches the Postgres raw SQL without escaping.
- **R-2** — honest queries keep their current results.
- **R-3** — mirror the SQLite sibling's `sql_quote()` rather than inventing a second pattern, and
  keep its comment explaining why a static bind chain cannot cover a variable-arity filter list.

---

## Design

Copy `sql_quote()` from
`adapters/diesel_sqlite/src/repositories/licensed_resources/licensed_resources_fetching.rs:37-45`
into the Postgres sibling and apply it to every string interpolated into the dynamic query: email,
role slug, and the UUIDs (the UUIDs are safe by type — `Display` cannot emit a quote — but going
through the same helper keeps the two adapters readable side by side).

`token_invalidation.rs` goes further: its queries have fixed arity, so all four of them (plus the
display-token `UPDATE`) move to bound params, which is what the SQLite sibling already does
(`diesel_sqlite/.../token_invalidation.rs:91-92, 183-184, 254-255`). That removes the last raw
`format!`-built SQL from the file and leaves one pattern instead of the three it had (raw,
hand-escaped, none).

### Bug fixed in passing

`RelatedAccounts::AllowedAccounts` emitted `acc_id = ANY(uuid,uuid)`. That is not valid Postgres —
`ANY` takes an array or a subquery — so the branch errored on **every** call, with one id or many:

```
ERROR:  syntax error at or near ","
ERROR:  op ANY/ALL (array) requires array on right side
```

Quoting alone cannot fix it, so the branch now emits `acc_id IN ('…','…')`, which is what the SQLite
sibling already produced. This changes behavior: a code path that always failed now works.

`IN ()` with an empty list would also be a syntax error, but an empty `AllowedAccounts` is not
constructible: `core/src/domain/dtos/profile/mod.rs:727-745` returns an error on `records.is_empty()`
before building the variant, and every other construction site passes `vec![single_id]`. No guard
added.

---

## Verification

`cargo fmt --all -- --check` clean, `cargo build --workspace` clean,
`cargo test --workspace --all` → **467 passed, 0 failed** (464 before + 3 new).

Three unit tests cover `sql_quote` directly, including the injection payload. `adapters/diesel_postgres`
has no DB integration harness, so the emitted SQL was checked against a disposable
`postgres:16-alpine` with the real table shape:

| Query form | `role.name` | Rows |
|---|---|---|
| old, interpolated | `x' OR '1'='1` | **2** — `admin` *and* `secret-role`; the role filter is bypassed and resources the caller never had are returned |
| new, `sql_quote` | `x' OR '1'='1` | **0** — payload treated as data |
| new, `sql_quote` | `admin` (honest) | 1 — R-2 holds |
| new, `IN ('…','…')` (AllowedAccounts) | — | 2 — branch works, where the old `ANY(…)` form errored |

`token_invalidation.rs`'s five converted statements were checked the same way — the honest lookups
still return their row, an injection payload in the username returns none, and
`jsonb_set(meta, '{token}', 'null'::jsonb)` (no longer passing through `format!`'s brace escaping)
still nulls the field.

Row 1 is the concrete impact: a horizontal privilege escalation in the profile's licensed resources,
not merely an error.

---

## Not covered

Validating role names at the ingress (`MyceliumProfileData::from_request`) is worth doing — the
repository layer should not be the only thing standing between a raw header and the query — but it is
a separate change with a different blast radius.
