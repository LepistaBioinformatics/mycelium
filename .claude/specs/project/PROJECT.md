# Mycelium

**Vision:** An open-source, multi-tenant API Gateway that enforces authentication, identity normalization, routing, and declarative security policy at the edge — without imposing business logic on downstream services.

**For:** Platform engineers and teams building multi-tenant, API-oriented systems that need a secure, extensible entry layer with composable authorization primitives.

**Solves:** The gap between a dumb reverse proxy and a full-blown business layer — Mycelium sits at the edge, handles coarse-grained authorization (gateway), injects identity context (Profile), and lets downstream services decide fine-grained access (FBAC) close to the resource.

---

## Goals

- Ship a stable GA release (v8.3.1 → v8.4.0 or higher) with zero production panics and full test coverage on critical paths.
- Maintain a clean hexagonal architecture with no domain leakage into adapters or ports.
- Provide a reliable MCP (Model Context Protocol) endpoint that exposes downstream service tools to LLM agents.
- Achieve OpenSSF Best Practices compliance and keep all known CVEs resolved within one sprint.

---

## Tech Stack

**Core:**

- Language: Rust (edition 2021, ≥1.70)
- Web framework: actix-web 4 + openssl
- Database ORM: Diesel (PostgreSQL)
- Cache: Redis (kv_db adapter)

**Key dependencies:**

- `shaku` 0.6 — compile-time dependency injection
- `mycelium-base` — `MappedErrors`, response kinds, `Parent/Children` DTOs
- `tera` — email/notification template rendering
- `lettre` — SMTP email delivery
- `utoipa` + `redoc` + `swagger-ui` — OpenAPI documentation
- `opentelemetry-otlp` + `tracing` — observability

---

## Scope

**Current focus (v8.x stabilization):**

- Eliminate remaining production panics and replace with `MappedErrors` propagation
- Strengthen test coverage for routing, security, and adapter layers
- Stabilize MCP endpoint for LLM tool-call integration
- Address HIGH/MEDIUM concerns from CONCERNS.md

**Explicitly out of scope:**

- Business logic — gateway enforces policy, never domain rules
- Hot-reload of arbitrary config (log level hot-reload is acceptable)
- Native rate limiting (delegated to upstream load balancers for now)
- Email address validation in the DTO layer (separate, later effort)

---

## Constraints

- **Technical:** Hexagonal architecture must be preserved — no adapter code in `core/`, no domain logic in `adapters/` or `ports/`.
- **Compatibility:** Public REST/RPC/MCP API must remain backwards-compatible within a major version.
- **Security:** Any change to the auth middleware, routing logic, or secret injection path requires a corresponding test. No merge without green CI.
- **Dependencies:** New dependencies require justification; prefer extending existing ones.
