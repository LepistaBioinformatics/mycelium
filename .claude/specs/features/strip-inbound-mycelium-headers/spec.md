# Fix: strip client-supplied `x-mycelium-*` headers before forwarding downstream

**Status:** Implemented — gates green (449 tests, 0 falhas), awaiting user UAT before commit
**Scope:** Medium (1 new file + 2 edited + tests)
**Severity:** High — authorization bypass in downstream services
**Branch:** shares the working tree with `fix/gateway-response-header-blocklist` (see *Commit plan*)

---

## Problem

`awc::Client::request_from(url, req.head())` (`awc-3.8.2/src/client/mod.rs:114-118`) copies
**every** header of the client request into the downstream request via `insert_header_if_none`.
The gateway never sanitizes that copy. It then overwrites *some* `x-mycelium-*` headers, selectively,
depending on the route's security group (`check_security_group.rs:99-188`):

| Security group | Overwritten by the gateway | Client-forged `x-mycelium-profile` |
|---|---|---|
| `Public` | nothing (`(downstream_request, None)`, line 110) | **forwarded verbatim** |
| `Authenticated` | `x-mycelium-email` only | **forwarded verbatim** |
| `Protected` | `x-mycelium-profile` (`insert`) | blocked |
| `ProtectedByRoles` | `x-mycelium-profile` (`insert`) | blocked |

So on any **Public** or **Authenticated** route, a client sends
`x-mycelium-profile: <base64(zstd(forged profile))>` and the gateway hands it to the downstream
service. Every SDK (py/js/go) decodes that header and trusts it as gateway-attested identity —
including `hasStaffPrivileges` / `isStaff`. That is a complete authorization bypass in the
downstream service, reachable through the gateway, requiring no Mycelium credential at all.

The exposure is wider than the profile. These have **no producer at all** on the proxy path, so a
client-supplied value is forwarded in *every* security group:

- `x-mycelium-scope`
- `x-mycelium-role`
- `x-mycelium-tenant-id`

And `x-mycelium-email` is forwarded on `Protected` / `ProtectedByRoles`, which only inject the
profile.

**Relation to the security scan:** `.claude/specs/security/findings.md` §3.1 records
"Spoofing de `x-mycelium-profile` não mitigado" as **conditional on the downstream service being
exposed directly, without the gateway**. That framing is wrong — this reclassifies it as reachable
*through* the gateway, which is precisely the path the SDK contract treats as trusted.

**Root cause is the same class as the response-header bug fixed alongside this one:** the trust
boundary was enforced by enumerating cases (per security group) rather than by denying by default
and re-granting explicitly.

---

## Requirements

| ID | Requirement |
|---|---|
| **R1** | Every header whose name starts with `x-mycelium-` is removed from the downstream request immediately after `request_from` copies the client headers, **unconditionally** — not per security group. |
| **R2** | The strip is prefix-based, not a list of known constants. A constant list re-breaks the day someone adds a new `DEFAULT_*_KEY` and forgets the filter — which is the exact failure mode of both bugs found in this session. `MYCELIUM_HEADER_PREFIX` is added to `settings.rs` next to the keys it governs. |
| **R3** | The strip runs inside `initialize_downstream_request`, **before** the gateway's own `insert_header` calls, so gateway-injected values survive. Pipeline order (`router/mod.rs:154-197`) guarantees it also precedes `check_security_group` and `inject_downstream_secret`. |
| **R4** | `x-mycelium-request-id` must still reach the downstream service. It is currently forwarded only as a side effect of the blanket copy, so after R1 it is **explicitly re-injected** from the upstream request. Its value is always gateway-generated: the app-level `wrap_fn` (`main.rs:805-812`) overwrites it with a fresh UUID on every inbound request before the router runs. |
| **R5** | The strip function is pure — `fn strip_inbound_mycelium_headers(headers: &mut HeaderMap)` — so it is unit-testable without constructing an `awc::Client`. It lives in its own file per screaming-architecture Rule 3. |

### Behavior change to validate in UAT

**`x-mycelium-connection-string` stops being forwarded downstream.** It is a client-supplied
credential, read by the gateway itself (`fetch_connection_string_from_request.rs:32`,
`mycelium_profile_data.rs:163`). Forwarding a user's bearer-equivalent secret to every downstream
service is a leak. Stripping it is deliberate.

Searched for consumers across the gateway (`core/`, `lib/`, `ports/`) and all three SDKs
(`mycelium-sdk-py/src`, `mycelium-sdk-js/src`, `mycelium-sdk-go`): each SDK only **declares** the
constant in its settings module — none reads the header. Services outside this monorepo were not
searched and cannot be. If some downstream does read it, this is the one behavior that will break,
and the fix is to re-inject it explicitly like the request id.

**Not affected:** `x-mycelium-security-group` is also stripped, but `check_security_group.rs:61-62`
re-inserts it unconditionally at the top of the function, before any branching, and that step runs
after `initialize_downstream_request` (`router/mod.rs:154` → `180`). Downstream still receives it.

### Tests (inline `#[cfg(test)] mod tests`)

| ID | Test |
|---|---|
| **T1** | A forged `x-mycelium-profile` is removed. |
| **T2** | The headers with no producer on the proxy path are removed: `x-mycelium-scope`, `x-mycelium-role`, `x-mycelium-tenant-id`, `x-mycelium-email`. |
| **T3** | `x-mycelium-connection-string` is removed (locks in the deliberate behavior change). |
| **T4** | Non-Mycelium headers are untouched: `authorization`, `content-type`, an app-custom header. |
| **T5** | Matching is prefix-based and case-insensitive — `X-Mycelium-Profile` is removed; `x-mycelium` (no trailing dash) and `x-my-header` are kept. |
| **T6** | A multi-valued non-Mycelium header keeps all its values (guards against removing by key when the map holds several entries). |

**These are unit tests, not regression tests.** The function is new, so there is no prior
implementation for them to fail against — none of T1-T6 would have caught the original bug, which
was the *absence* of this step in the pipeline rather than a defect inside it. They lock the
unit's contract (prefix boundary, case-insensitivity, multi-value) against future edits.

A pipeline-level regression test — forge `x-mycelium-profile`, run `initialize_downstream_request`
on a `Public` route, assert the header is absent from the resulting `ClientRequest` — would be the
only test proving the bypass is closed end to end. **Deliberately not written (user decision,
2026-07-27): unit tests only.** It would need `Route` / `ApiConfig` / `web::Data<Client>` fixtures
that do not exist in the crate. The strip point and pipeline order were verified by reading
`router/mod.rs:154-197` and `check_security_group.rs:61-62`, not by test.

### Gate

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all
```

---

## Commit plan

Both this fix and `fix/gateway-response-header-blocklist` are uncommitted in the same working
tree (per `commit-validation.md`, neither may be committed before user UAT). The file sets are
disjoint except `router/mod.rs`:

- **Response fix:** `build_the_gateway_response.rs`, `lib/http_tools/src/settings.rs`,
  `router/mod.rs` (call site)
- **This fix:** `router/strip_inbound_mycelium_headers.rs` (new),
  `router/initialize_downstream_request.rs`, `router/mod.rs` (module decl),
  `lib/http_tools/src/settings.rs`

After UAT they should become two commits, and can be split into two PRs by branching each from
`develop` and cherry-picking. This one is the security fix and warrants its own PR and changelog
entry.

---

## Out of scope (user decision, 2026-07-27)

- **Profile signing (HMAC gateway↔SDK).** Would eliminate the whole class rather than this
  instance, but changes the contract across all four SDKs. Deferred.
- **Trusted-proxy validation in `resolve_client_ip`** (findings.md §1.3). Same class of bug —
  trusting a client-supplied header — but needs a new config surface, and an empty default would
  change the client IP seen by downstreams when running behind Traefik/Dokploy. Its own task.
