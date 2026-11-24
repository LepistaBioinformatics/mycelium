# Summary — 002 REST beginners account creation with internal tokens

**Status:** implemented, all gates green, **awaiting user UAT before commit**
(`.claude/rules/commit-validation.md`).
**Branch:** `fix/rest-beginners-internal-provider` off `origin/develop` (`f3e71a9e`).

## What changed

| File | Change |
|---|---|
| `ports/api/src/middleware/resolve_issuer_from_provider.rs` | **new** — single resolver: `None` provider → `MYCELIUM_PROVIDER_KEY` (`"mycelium"`), `Some` → the configured issuer. 2 unit tests. |
| `ports/api/src/middleware/mod.rs` | registers/re-exports the new module |
| `ports/api/src/rest/role_scoped/beginners/account_endpoints.rs` | `create_default_account_url` uses the resolver (drops the `400 "Invalid provider"` dead-end) and returns `err.error_response()` for `GatewayError` instead of a blanket 500 |
| `ports/api/src/rpc/dispatchers/beginners.rs` | `BEGINNERS_ACCOUNTS_CREATE` uses the same resolver in place of its inline `if/else`; the now-unused `MYCELIUM_PROVIDER_KEY` import is gone |

Net: −32/+24 lines across the three edited files. The REST route now matches the RPC one. The RPC
route keeps its outcome and its `INTERNAL_ERROR` code; the only change there is the error *message*
when an external issuer fails to resolve — now a generic "Unable to resolve the identity provider
issuer" instead of the raw secret-resolver error text, so resolver internals no longer reach the
client.

## Root cause (correction to the issue)

Issue #172 says "the divergence must be **upstream** of the shared handler" and lists the cause as
"not yet traced". That is wrong, and the "Not yet traced" section will send the next reader hunting
per-route middleware that does not exist.

Both routes call the same `check_credentials_with_multi_identity_provider` and get the **same**
tuple back. `get_email_or_provider_from_request` returns `(Some(email), None, token)` for the
`mycelium` issuer — `external_provider = None` *is* the internal-provider signal, not a failure.
The divergence was two lines in the two handler bodies branching differently on that `None`.
`0c4f5163` (magic-link) fixed the RPC branch and left REST behind.

## Verification

- `cargo fmt --all -- --check` — clean
- `cargo build --workspace` — clean; also `cargo build -p mycelium-api --no-default-features --features standalone` (the mode the issue reproduces in) — clean
- `cargo test --workspace --all` — all green, including the 2 new
  `middleware::resolve_issuer_from_provider::tests`
- Not exercised end-to-end: `ports/api` has no handler-test harness (only `router/` has
  `actix_web::test` modules), so the live magic-link → `POST /_adm/beginners/accounts` path is
  UAT, not automated.

## Found but deliberately not fixed

None of these change this fix; all are real; each deserves its own issue.

1. **`admin_jsonrpc_post` falls back to an anonymous profile on `Unauthorized(_)`, not just
   `Forbidden(_)`** (`ports/api/src/rpc/handlers.rs:250-268`). The code comment only justifies the
   `Forbidden` case (authenticated user with no account yet, so `beginners.accounts.create` is
   reachable). The `Unauthorized` arm means a request with **no token at all** reaches every
   dispatcher with a nil-UUID profile. Most methods guard on profile fields and will deny, but the
   set that does not has not been enumerated. Security-relevant; own issue.
2. **Four TOTP handlers in `ports/api/src/rest/role_scoped/beginners/user_endpoints.rs`**
   (`:690`, `:766`, `:835`, `:917`) have the same `GatewayError → 500` flattening this task fixed
   on the account route. Same class of bug, four more routes, not named by #172.
3. **REST `GET /_adm/beginners/accounts` declares `204 Not found`** in its utoipa block, but the
   `MyceliumProfileData` extractor 403s ("User was authenticated but has not an account",
   `recovery_profile_from_storage_engines.rs:65`) before the handler body runs, so that branch is
   unreachable for account-less users. This is the "detection is broken" half of #172's body.
   Fixing it means deciding whether that route takes an optional-profile extractor — a design call
   for the user; #172's "Expected" section only covers the create route.
4. **`Some("mycelium")` makes the user-NotFound path register `Provider::External("mycelium")`**
   (`create_account_from_existing_user.rs:190`), which is semantically bogus — the internal
   provider is `Provider::Internal`. Not introduced here: it is already live on the RPC path, so
   the REST route now mirrors it rather than adding anything new. Correcting it means changing
   `create_user_account`'s `provider: Option<String>` to distinguish internal from external.
   Unreachable in practice for magic-link users (the verify flow creates the user first).

## Next step

User UAT against a standalone instance: magic-link login, then
`POST /_adm/beginners/accounts {"name":"<email>"}` → expect `201` with a `"user"` account, and the
same account that `beginners.accounts.create` would return. Then commit + PR into `develop`
(never push to `develop` directly — `.claude/rules`, branch protection).
