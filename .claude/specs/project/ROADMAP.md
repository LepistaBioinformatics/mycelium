# Roadmap

**Current Milestone:** M1 — Stability & Safety
**Status:** In Progress

---

## M1 — Stability & Safety

**Goal:** Eliminate all production panic paths, fix HIGH/MEDIUM concerns from CONCERNS.md, and bring critical path test coverage to an acceptable baseline. Makes the codebase safe to release as GA.

**Target:** v8.4.0-GA (no explicit date)

### Features

**Panic Elimination** - COMPLETE

- Replace all `.unwrap()` / `panic!()` in production paths with `MappedErrors`
- Notifier email parsing (NPANIC-01..03) — done `b41b381c`
- Tera template init (NPANIC-04..05) — done `b41b381c`
- Service host selection (NPANIC-06) — done `b41b381c`

**Forwarded Header Compliance** - PLANNED

- Implement RFC 7239 `Forwarded` header parsing in `router/mod.rs`
- Validate `X-Forwarded-For` only from trusted proxies
- Replace the `TODO` at `router/mod.rs:53-55`

**JWT Secret Minimum Length Validation** - PLANNED

- Add ≥256-bit check in `lib/http_tools/src/functions/encode_jwt.rs`
- Return `MappedErrors` on undersized secrets

**Router & Auth Middleware Tests** - PLANNED

- Unit tests: source IP validation, method permission checks, secret injection, header filtering, URL building
- Integration tests: all REST/RPC/MCP endpoint handlers via `actix-rt::test`

**mTLS Client Certificate Authentication** - PLANNED

- Implement client cert validation in `inject_downstream_secret.rs`
- Unblock the `TODO` in `core/src/domain/dtos/http_secret.rs:66`

---

## M2 — Observability & Resilience

**Goal:** Improve operational visibility and harden the database and async initialization layers.

### Features

**Database Integration Tests** - PLANNED

- testcontainers-based PostgreSQL tests for all Diesel repositories
- Catch migration regressions before production

**Connection Pool Limits** - PLANNED

- Verify and document `DieselDbPoolProvider` pool size configuration
- Add hard limits to prevent DB exhaustion

**Async Config Initialization** - PLANNED

- Replace `lazy_static! + Mutex` in `lib/config/src/settings.rs` with `tokio::sync::OnceCell`
- Eliminate lock contention on startup

---

## M3 — Auth Evolution

**Goal:** Reduce friction for end users with passwordless login and enable Mycelium to run with zero external dependencies for simpler deployments.

### Features

**Magic Link (Passwordless Login)** - PLANNED

- Generate time-limited, single-use tokens stored in Redis (or in-memory for standalone)
- Send token via email using the existing notifier adapter
- New endpoints: `POST /auth/magic-link/request` and `GET /auth/magic-link/verify?token=`
- Token verified → session issued identically to password-based login
- Configurable TTL and domain allowlist

**Standalone Mode** - PLANNED

- Run Mycelium with zero external dependencies: no PostgreSQL, no Redis, no Vault
- Auto-provision a local SQLite database via a new `sqlite` Diesel adapter
- In-memory KV store replaces Redis (`mem_db` adapter already exists — wire it)
- Secrets loaded from local encrypted file instead of Vault
- Single binary, zero infra: ideal for local dev, edge deployments, and small teams
- Activated via config flag `mode = "standalone"`

---

## M4 — Gateway Policy

**Goal:** Give operators fine-grained control over downstream API consumption with quota enforcement at the gateway layer.

### Features

**Rate Limiting for Downstream APIs** - PLANNED

- Per-route and per-tenant configurable quotas (req/min, req/day)
- Quota config stored in gateway DB alongside route definitions
- Enforcement via middleware before the request is forwarded downstream
- HTTP 429 with `Retry-After` header on limit exceeded
- Tenant-level override support (e.g., premium tenants get higher quotas)

**Gateway-level Inbound Rate Limiting** - PLANNED

- Per-IP and global limits to protect the gateway itself
- Configurable via gateway settings

---

## M5 — MCP & Ecosystem

**Goal:** Stabilize and extend the MCP endpoint for production LLM agent use cases.

### Features

**MCP Tool Schema Validation** - PLANNED

- Validate tool input against downstream OpenAPI schema before forwarding
- Return structured JSON-RPC errors on schema mismatch

**MCP Authentication Scoping** - PLANNED

- Per-tool auth context injection
- Allow downstream services to declare required scopes per MCP tool

**Guest Role Endpoints** - PLANNED

- Complete the `TODO` implementation in `guest_role_endpoints.rs`
- Cover with integration tests

---

## M6 — Mycelium Hub

**Goal:** Enable multiple Mycelium instances to federate identities, so a user authenticated in one node is recognized by all nodes in the hub — without duplicating user stores.

### Features

**Inter-node Identity Federation** - PLANNED

- Nodes discover each other via a hub registry (DNS-SD or static config)
- Identity tokens issued by any node carry a `node_id` claim
- Receiving node validates the token signature against the issuing node's public key (fetched on first contact, cached with TTL)
- No shared database required — each node keeps its own store; only token validation is federated
- Trust model: explicit allowlist of trusted node URLs per instance

**Hub Node Registry** - PLANNED

- New endpoint: `GET /hub/nodes` — list trusted peer nodes
- New endpoint: `POST /hub/nodes` — register a peer (admin only)
- Peer health checked periodically; stale nodes marked inactive

**Cross-node Profile Propagation** - PLANNED

- Profile (identity context) injected by the originating node and forwarded as a signed header
- Receiving node verifies signature and passes Profile to downstream as usual
- Downstream services remain unaware of the multi-node topology

---

## Future Considerations

- **WebAuthn / Passkey** — hardware-backed passwordless auth (Touch ID, YubiKey); add after magic link proves adoption
- JSONB column schema validation (prevent schema drift in `up.sql`)
- Config hot-reload for non-critical settings (log level, rate limits)
- `cargo audit` in CI + Dependabot integration for dependency CVEs
- Route matching optimization (HashMap/trie) for high service-count deployments
- GA version release: promote out of `8.3.1-beta.x` once M1 is complete
