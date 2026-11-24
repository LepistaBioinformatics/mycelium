# Fix: Discovery operations listing (slow + wrong results)

**Status:** Specified — executing
**Size:** Medium (one module, `ports/api/src/openapi_processor/`; design inline)
**Branch:** `fix/discovery-operations-listing` (from `develop`)

## Problem

The webapp Discovery screen (`gatewayManager.tools.list`, REST `GET .../tools`) is slow even
though every downstream OpenAPI document is already in memory (`ServiceOpenApiSchema`).
`list_operations` (`openapi_operations.rs`) does, on every request:

1. **Per returned record**, `serde_json::to_value(operations_database.docs.clone())` — clones
   and serializes the OpenAPI documents of **every** service to read one service's document.
   A 20-record page serializes every document 20 times.
2. Clones and re-sorts the whole operations list.
3. Resolves `$ref`s three times, cloning the operation JSON on each pass.
4. Falls back to merging the **whole service document** into the record when the operation has
   no `operationId` or its input refs cannot be resolved — inflating the response.

It also returns wrong results:

5. `count` is the page length (`records.len()`), not the number of matches, so the webapp pager
   cannot tell whether more pages exist.
6. Without a query every score is 0, below the default `scoreCutoff` (5), so listing without a
   search term returns nothing.

## Requirements

- **DISC-01** Each service document is serialized to JSON once, when the documents are loaded,
  not per request or per record.
- **DISC-02** Operations are sorted once, at load time; a request does not clone the list.
- **DISC-03** `$ref` resolution works in place on the record (no clone per pass); the limits
  (3 passes, depth 15) stay.
- **DISC-04** A record never carries the whole service document; when input refs cannot be
  resolved it carries the operation alone.
- **DISC-05** `count` is the number of operations matching the method/query filters, before
  pagination.
- **DISC-06** Without a query, every operation (after the method filter) is listed in the stable
  load order; `scoreCutoff` only applies when there is a query.
- **DISC-07** Response shape unchanged otherwise (`records[].operation`, `records[].score`,
  `pageSize`, `skip`), so the webapp keeps working without changes.

## Verification

- Unit tests: count vs. page; empty query lists everything; score cutoff still filters a query;
  a record does not contain the service document's `paths`.
- A timing comparison on a synthetic catalog (before vs. after), reported in the PR.
- Gate: `cargo fmt --all -- --check`, `cargo build --workspace`, `cargo test --workspace --all`.
