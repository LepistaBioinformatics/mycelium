# Feature: Native error codes without database registration

**Status:** Implemented (NEC-01..NEC-06) — gate passing (1137 tests; standalone and
postgres-only builds); live check on the standalone instance with no CLI run: `GET /error-codes`
returns the 30 non-internal natives (37 natives, 7 internal). Book chapter 17 updated + pt-BR.
SPEC_DEVIATION (NEC-05): registering refuses any custom code under a native prefix (`MYC`),
because registration always stores `error_number = 0`, so a `(prefix, code)` collision check
could never fire. Pending user approval before commit.
**Size:** Medium (error-code use cases + both SQL adapters' list filter; design inline)

## Problem

Mycelium's native error codes (`NativeErrorCodes`, `MYC00001`…, defined with message, details
and `is_internal` in `core/src/domain/dtos/native_error_codes.rs`) only reach the `error_code`
table when an operator runs the CLI `batch_register_native_error_codes`. Every error-code screen
reads the table only, so on an instance where nobody ran the CLI (e.g. the standalone test
instance) the dashboard and the public `/error-codes` screen show none of them.

## Decision

- **D-01** The code is the source of truth for native codes. Listing and reading merge the
  in-code natives (`NativeErrorCodes::to_error_codes()`) with the custom codes stored in the
  database. Native rows already in the database (from earlier CLI runs) are ignored when listing
  and reading, so nothing is shown twice and an edited copy can never contradict the code.
  The CLI keeps working (harmless) but is no longer needed.

## Requirements

- **NEC-01** A pure, unit-tested helper (core) that, given filters (`prefix`, `code`,
  `is_internal`), returns the matching in-code natives sorted by `(prefix, error_number)`.
- **NEC-02** The error-code fetching port's `list` can exclude native rows (new parameter or a
  dedicated filter; implement in the Postgres and SQLite adapters, mock updated). Custom codes
  are listed with the same filters as today.
- **NEC-03** `list_error_codes` (systemManager, RPC + REST) and `list_public_error_codes` return
  natives first, then custom codes, as one paginated result: `count` = natives matching +
  customs matching; `skip`/`page_size` slice the combined sequence (natives page served from
  memory, the rest from the database with the adjusted skip/size). The public use case keeps
  forcing `is_internal = false` and the page-size clamp.
- **NEC-04** `get_error_code(prefix, code)` (systemManager.errorCodes.get and REST) returns the
  in-code native when `(prefix, code)` is a native, without touching the database.
- **NEC-05** Writes that would act on a native through the database (update message/details,
  delete) are refused with a clear, expected error (natives are defined in code). Registering a
  custom code whose `(prefix, code)` collides with a native is refused too.
- **NEC-06** Tests: pagination across the native/custom boundary (page fully native, page
  straddling, page fully custom), filters, `count`, public never returns internal natives,
  get-by-code for a native, refused writes.

## Verification

Full gateway gate (fmt, workspace build, workspace tests, standalone and postgres-only builds);
`curl /error-codes` on the standalone instance shows the non-internal natives with no CLI run.
