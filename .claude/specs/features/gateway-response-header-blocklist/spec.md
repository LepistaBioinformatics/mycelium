# Fix: gateway response headers must use a blocklist, not an allowlist

**Status:** Implemented — gates green, awaiting user UAT before commit
**Scope:** Medium (2 files + tests, no architectural change)
**Branch:** `fix/gateway-response-header-blocklist`

---

## Problem

`ports/api/src/router/build_the_gateway_response.rs` builds a single `Vec<String>`
containing `route_key` + `FORWARDING_KEYS` + `FORWARD_FOR_KEY` + `DEFAULT_PROFILE_KEY`
+ `MYCELIUM_SERVICE_NAME`, then copies from `downstream_response` **only** the headers
present in that list.

The design intent, documented at `lib/http_tools/src/settings.rs:88-92`, is the
opposite: *"Such keys are used to map the headers that should be removed from the
downstream response before stream it back to the client."* The implementation inverted
the predicate. Consequences:

1. **Every legitimate downstream response header is dropped.** `Content-Type`,
   `Cache-Control`, `ETag`, `Location`, `Content-Disposition`, `WWW-Authenticate`,
   `Set-Cookie`, and all application-custom headers never reach the client.
2. **The headers that *do* pass are exactly the ones that should not.** `route_key` is
   the *name of the injected downstream secret header*
   (`inject_downstream_secret.rs:103`). `x-mycelium-profile` carries the caller's
   authorization context. Under the current allowlist, a downstream service that echoes
   either header back has it forwarded verbatim to the client.

Observed symptom: SSE responses from `crab-shell-proxy` arrive without
`Content-Type: text/event-stream`, so `EventSource` and OpenAI-compatible SDKs buffer
the whole stream instead of dispatching per event.

---

## Blast radius (investigation — request item 4)

**This is not streaming-specific. It affects every proxied response.**

- `route_request` is wired as `App::default_service` only
  (`ports/api/src/main.rs:814`). So the bug covers 100% of traffic proxied to
  downstream services, and 0% of Mycelium's own handlers (`/_adm/**`, `/doc/**`,
  `/health`), which return normal actix `Responder`s and set their own `Content-Type`.
  That split is why this went unnoticed: the webapp talks to Mycelium's own endpoints.
- **Nothing in the pipeline masks it.** There is no `Compress` and no `DefaultHeaders`
  middleware in the app (`main.rs:564-573`, `658`); the only wraps are `cors`,
  `NormalizePath`, `RequestTracing`, `TracingLogger`, `Logger`, and a `wrap_fn` that
  injects the request id. `HttpResponseBuilder::streaming` produces
  `BodySize::Stream`; the actix-http h1 encoder writes only the headers explicitly set,
  plus `Date` and `Transfer-Encoding: chunked`
  (`actix-http-3.13.1/src/h1/encoder.rs:92-156`). Actix never supplies a default
  `Content-Type`.
- Therefore **JSON responses proxied through the gateway are served today with no
  `Content-Type` at all**. `fetch(...).json()` and most HTTP clients parse anyway
  (they don't require the header), which is why only the SSE case produced a visible
  failure. Redirects (`Location` dropped) and file downloads (`Content-Disposition`
  dropped) are silently broken by the same code path.

---

## What must NOT change (verified constraints)

- **`Content-Encoding` must keep being forwarded.**
  `initialize_downstream_request.rs:203` calls `.no_decompress()`, so awc's
  `Decompress` wrapper is a pass-through and the body reaches the client still
  compressed. Blocking `Content-Encoding` would corrupt every gzip/brotli/zstd
  response. It is *not* added to the blocklist.
- **`Content-Length` must keep being forwarded.** For `BodySize::Stream` the h1
  encoder already drops it (`skip_len = true`, `encoder.rs:63,153`), but it is
  deliberately *retained* for `304 Not Modified` (`encoder.rs:80-86`), which is the
  RFC 7232 §4.1 behavior. Blocklisting it would break that. It is not added.
- **`FORWARDING_KEYS` already contains the complete hop-by-hop set** requested in the
  task (`Host`, `Connection`, `Keep-Alive`, `Proxy-Authenticate`,
  `Proxy-Authorization`, `Te`, `Trailers`, `Transfer-Encoding`, `Upgrade`) —
  `settings.rs:93-103`. **No new entries are needed.** The constant has exactly one
  consumer workspace-wide (`build_the_gateway_response.rs`), so restructuring its use
  cannot affect the request path.
- No change to authentication, RBAC, or secret injection.

---

## Requirements

| ID | Requirement |
|---|---|
| **R1** | All headers from `downstream_response` are copied to the gateway response, except those in the blocklist. |
| **R2** | The blocklist is composed of two semantically distinct, separately named sets: **(a) hop-by-hop headers** — `FORWARDING_KEYS`, blocked because RFC 7230 §6.1 forbids proxy forwarding; **(b) gateway-injected artifacts** — the whole `MYCELIUM_HEADER_PREFIX` namespace, plus `route_key` (downstream secret header name), `FORWARD_FOR_KEY` and `RFC7239_FORWARDED_KEY` — blocked because they are request-direction headers the gateway injects; if a downstream echoes one back, forwarding it leaks internal context to the client. |
| **R2.1** | The Mycelium set is matched **by prefix**, not by listing keys. A first pass enumerated `DEFAULT_PROFILE_KEY` / `DEFAULT_REQUEST_ID_KEY` / `MYCELIUM_SERVICE_NAME` and thereby let `x-mycelium-email`, `x-mycelium-security-group`, `x-mycelium-connection-string`, `x-mycelium-scope`, `x-mycelium-role` and `x-mycelium-tenant-id` through — an echoing downstream would have leaked the authenticated user's email, the route's authorization config, and the user's connection-string credential to the client. Caught by an automated security review of the commit. Same enumeration-vs-prefix failure this spec warns about elsewhere; the prefix is now the only rule. |
| **R3** | Matching is case-insensitive (HTTP header names are case-insensitive). |
| **R4** | Blocking `DEFAULT_REQUEST_ID_KEY` also removes an ordering hazard: the gateway inserts its own request id at the top of the function, and an echoed downstream value would otherwise overwrite it via `insert_header`. |
| **R5** | `build_the_gateway_response` takes `StatusCode` + `&HeaderMap` instead of `&DownstreamResponse`, so it is unit-testable. It already only reads status and headers. The borrow ends before `mod.rs` moves `downstream_response` into `.streaming(...)`. |
| **R6** | `settings.rs` doc comment on `FORWARDING_KEYS` is corrected to state it is the hop-by-hop blocklist, matching the now-correct implementation. |
| **R7** | Forwarding uses `append_header`, not `insert_header`. `HeaderMap::iter` yields one entry per value, so an insert collapses multi-valued headers (`Set-Cookie`, `Vary`, `Link`, `WWW-Authenticate`) to their last value. Under the old allowlist those headers were dropped entirely, so this only becomes reachable once R1 lands — forwarding a partial `Set-Cookie` set would be worse than dropping it. Appending is safe: `HttpResponse::build` starts with an empty map and the only pre-loop insert (`DEFAULT_REQUEST_ID_KEY`) is itself blocklisted. |

### Tests (inline `#[cfg(test)] mod tests`, per CONVENTIONS)

| ID | Test |
|---|---|
| **T1** | `Content-Type: text/event-stream` on the downstream response reaches the client intact. |
| **T2** | Hop-by-hop headers (`Connection`, `Transfer-Encoding`) are removed. |
| **T3** | An arbitrary application header (e.g. `x-crab-shell-session`) passes through. |
| **T4** | Gateway-injected artifacts echoed by downstream are stripped: `route_key`, `x-mycelium-profile`, `x-mycelium-service-name`. |
| **T5** | An echoed `x-mycelium-request-id` does not override the gateway's own value. |
| **T6** | Blocklist matching is case-insensitive (`CONNECTION`, `Content-Type` in mixed case). |
| **T7** | Two `Set-Cookie` values on the downstream response both reach the client (regression guard for R7 — verified failing with `insert_header`, passing with `append_header`). |

| **T8** | The whole `x-mycelium-` namespace is stripped when echoed by the downstream — email, security-group, connection-string, scope, role, tenant-id, and a never-seen `x-mycelium-not-yet-invented` (guards R2.1). |

**All eight are verified regression tests.** The filter was temporarily reverted to the original
allowlist (inverted predicate + `insert_header`) and the suite re-run: **0 passed, 7 failed**.
Restored: 7 passed. T8 was verified separately against the enumerated blocklist that preceded the
prefix match: **failed**, then passed once the prefix rule landed. Every test in this file fails
against the code it replaces.

Assertions read headers via `builder.finish()` → `res.headers()`, since
`HttpResponseBuilder` exposes no getter. `finish()` must be called once.

### Gate

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all
```

---

## Deferred

- **Dynamic hop-by-hop tokens (RFC 7230 §6.1):** a downstream may list additional
  connection-scoped header names in its own `Connection:` header; a strict proxy would
  parse and strip those too. Out of scope here — the static list covers the observed
  case.
- **`Content-Length` + chunked interplay for HTTP/2 responses:** the h2 encoder path
  was not audited; h1 is verified correct.
