# Quick 004 — Block sign-in until the staff bootstrap is claimed

**Date:** 2026-10-22

A user who signed in before the staff bootstrap got a regular account; running
the bootstrap later with the same email did not make them Staff
(`claim_staff_bootstrap` uses `get_or_create_user_account`, which returns the
existing User account). The user asked: while nobody has done the bootstrap,
nobody can sign in.

## Change

- `ports/api/src/middleware/staff_bootstrap_gate.rs`:
  `sign_in_blocked_by_staff_bootstrap(req)` — open when no
  `staff_bootstrap_secret` is configured; blocked while the bootstrap is
  pending; fails closed when the state can't be read; process-wide
  `StaffBootstrapClaimed` latch (app data, `main.rs`) stops DB reads once
  claimed.
- Applied in: `check_credentials_with_multi_identity_provider` (every
  authenticated request, any IdP → 403), `/magic-link/request`,
  `/magic-link/verify`, password `/login`, hosted login `/authorize`
  (error page with its own message) and `/token`. The bootstrap flow
  (`/instance/bootstrap*`) calls the use cases directly and stays open.

## Verify

Unit tests (no secret → open, latch → open without DB, unreadable → closed).
Gateway gates: fmt, build (full + standalone), 548 tests pass. Throwaway
standalone with the secret: while pending — authorize shows the message,
magic-link request/verify and /login 403, /token refused; bootstrap
completes and the claimer is `isStaff=true`; afterwards authorize 200 and a
second user can request a magic link.

## Known gaps (not changed)

- Instances that already have accounts and an unclaimed bootstrap keep the
  claim bug (claimer's existing account is not upgraded to Staff), and now
  lock everyone out until the claim. `dokploy/mycelium-api-gateway/config.toml`
  sets `staffBootstrapSecret` from env.
- Unauthenticated `beginners.profile.get` over RPC returns a zeroed profile
  (200) instead of 401 — pre-existing, worth a look.
