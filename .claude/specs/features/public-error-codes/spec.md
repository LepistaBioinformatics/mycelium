# Feature: Public error codes

**Status:** Implemented (PEC-01..PEC-07) — gateway gate passing (1118 tests; standalone and
postgres-only builds), webapp gate passing (88 unit, 7 e2e); verified with curl against the
standalone instance without a token. Pending user approval before commit.
Notes: an empty result is `200` with `records: []` (adapters always paginate), so the webapp
treats `204` and an empty page alike; `pageSize` is clamped to 1..100 and a negative `skip` to 0;
`code` is always null from the repository, so the webapp formats `PREFIX00023` itself.
**Size:** Medium (one use case + one public REST endpoint in the gateway; one public screen in
the webapp; design inline)
**Branch:** gateway `fix/discovery-operations-listing` working tree (separate commit); webapp
`feat/ui-shadcn` working tree (separate commit)

## Context

Error codes with `internal = false` exist to be shown to the people calling the gateway and its
downstream services, but they can only be listed by a signed-in SystemManager
(`systemManager.errorCodes.list`, REST `role_scoped/system_manager/error_code_endpoints.rs`).
Users who receive an error (e.g. `MYC00023`) have no way to look it up.

## Decisions (user, 2026-11-23)

| ID | Decision |
|---|---|
| D-01 | The public endpoint is **always on** (no config switch). It only ever returns error codes with `internal = false`. |
| D-02 | The webapp gets a public route `/error-codes` (no sign-in), linked from the Home footer and the login page. |

## Requirements

### Gateway

- **PEC-01** New use case `list_public_error_codes(prefix, code, page_size, skip, repo)` in
  `core/src/use_cases/gateway/` (no profile, no role): calls the fetching port with
  `is_internal = Some(false)`, always. Unit-tested with a mocked `ErrorCodeFetching` asserting
  the `is_internal` argument is `Some(false)`.
- **PEC-02** Public REST endpoint `GET /error-codes` (scope registered next to `/health` and
  `/app-config` in `main.rs`), query `prefix`, `code`, `pageSize`, `skip`; no authentication
  (`security(())` in utoipa). Responds `200` with the paginated records (same shape as the
  role-scoped list), `204` when nothing matches, `500` on repository failure.
- **PEC-03** `pageSize` is capped (100) so the public endpoint cannot be asked for unbounded
  pages.
- **PEC-04** OpenAPI document lists the endpoint; book chapter on error codes (if any) mentions
  it; pt-BR po updated if the book changes.

### Webapp

- **PEC-05** Public route `/error-codes` (outside the dashboard, no auth), screen
  `screens/PublicErrorCodes/`: header, search bar (letters → prefix, digits → code, e.g.
  `MYC 23`), list of codes (`PREFIX00023` chip, message, details), pager, empty and
  unavailable states.
- **PEC-06** Links from the Home footer and the login page; i18n in en, pt-br, es.
- **PEC-07** Fetch through a small public client (no token), like `services/app-config.ts`.

## Verification

- Gateway: unit test for PEC-01; full gate (`cargo fmt --check`, `cargo build --workspace`,
  `cargo test --workspace --all`); `curl` against the standalone test instance without a token.
- Webapp: `yarn build`, `yarn lint`, `yarn test`; Cypress screenshot of `/error-codes` signed out.
