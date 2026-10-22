# Quick 003 — Identity resolution logs to debug, one access line per request

**Date:** 2026-10-14

Resolving the requester (credentials, JWKS, email, profile, cache hits,
security group, header injection) logged ~10 `info` lines per request, for
downstream routes and internal endpoints alike. Only the final outcome, with
who made the request, should be logged at `info`.

## Change

- Intermediate identity/profile steps → `debug` in
  `check_credentials_with_multi_identity_provider.rs`,
  `recovery_profile_from_storage_engines.rs`,
  `fetch_and_inject_profile_from_token_to_forward.rs`,
  `fetch_and_inject_email_to_forward.rs`, `router/check_security_group.rs`
  ("Service not found" → `warn`), and the RPC "RPC request" start line.
- `models/requester_identity.rs`: redacted email + account id stored in the
  request extensions where identity resolves.
- `middleware/log_served_request.rs` (actix `from_fn`): one `info`
  "Request served" line per request — method, path, status, duration,
  request id, client IP, requester account id and redacted email. Skips
  `/health`, `/doc/swagger`, `/doc/redoc`. Replaces the actix `Logger`
  access log (same exclusions) so there is still one line per request.

## Verify

Gates: fmt, build, 545 tests pass. Live standalone run: an authenticated
`/_adm/beginners/profile` and `/_adm/rpc` each produce a single
"Request served" line with `requester.email`; `/health` produces none.
