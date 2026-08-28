# 002 — REST `POST /_adm/beginners/accounts` rejects internal (magic-link) tokens

**Issue:** [#172](https://github.com/LepistaBioinformatics/mycelium/issues/172)
**Mode:** quick (3 files, one behavioural change)
**Branch:** `fix/rest-beginners-internal-provider` (off `origin/develop` @ `f3e71a9e`)
**Reported on:** 9.0.0-rc.5, standalone mode, internal magic-link identity

## Problem

With a valid internal (magic-link) session JWT:

| Call | Before |
|---|---|
| `POST /_adm/beginners/accounts` (REST) | `400 {"msg":"Invalid provider"}` |
| `POST /_adm/rpc` → `beginners.accounts.create` | `200`, creates a `"user"` account |

Same token, same use-case, opposite outcome. A client following the REST beginners API cannot
create its own account and has to fall back to `/_adm/rpc`.

## Root cause — the issue's hypothesis is wrong

The issue guesses the divergence is *upstream* of the shared handler, in per-route middleware.
It is not. Both routes call the same `check_credentials_with_multi_identity_provider` and receive
the **identical** tuple. `get_email_or_provider_from_request` returns
`(Some(email), None, token)` when the issuer is `mycelium` — `external_provider = None` is the
*correct signal for the internal provider*, not a failure.

The two handlers then branch differently on that same `None`:

- `ports/api/src/rest/role_scoped/beginners/account_endpoints.rs:131` → `400 "Invalid provider"`
- `ports/api/src/rpc/dispatchers/beginners.rs:111` → `MYCELIUM_PROVIDER_KEY.to_string()`

The drift is not hypothetical: `0c4f5163` (magic-link, M3) added the fallback on the RPC side and
left the REST route behind.

## Scope

1. Extract the `Option<ExternalProviderConfig> -> issuer` resolution into one shared function so
   the two transports cannot drift again, with unit tests (the pure function is testable;
   the handler is not — `ports/api` has no handler-test harness, only `router/` module tests).
2. REST route uses it → internal tokens resolve to the `mycelium` issuer instead of a 400.
3. REST route maps `GatewayError` through `ResponseError::error_response()` instead of
   flattening every auth failure into a 500. This covers the issue's second "Expected" clause:
   the error now reflects the real reason (401 for a bad token) rather than "Invalid provider".

## Done when

- `POST /_adm/beginners/accounts` with an internal magic-link JWT creates the user account,
  matching `beginners.accounts.create`.
- An invalid/absent token on that route returns 401, not 500.
- `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all` green.
