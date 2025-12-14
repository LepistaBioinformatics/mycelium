# Mycelium Authorization Model

This document describes the **Mycelium authorization model**, its
principles, formal classification, and architectural decisions. It is strictly
technical and does not address aspects of project governance or maintenance.

---

## Overview

Mycelium adopts a **contextual authorization model**, designed for
multi-tenant and API-oriented environments. Unlike approaches
based exclusively on roles (RBAC), Mycelium evaluates identity attributes,
resource scope, and request context at execution time.

Formally, the model fits as **FBAC (Fine-grained / Feature-based
Access Control)**, using RBAC only as a complementary primitive.

---

## Design Principles

* Clear separation between authentication, identity enrichment, and
  authorization
* Explicit access decision close to the resource
* Progressive reduction of capabilities
* Composable and deterministic primitives
* Avoid rigid global policies

---

## Layered Authorization

### Gateway (Coarse-grained)

At the gateway, Mycelium applies declarative controls per route, such as:

* Public or protected group
* Minimum required roles
* Permissions associated with roles

These checks determine whether the request can or cannot be routed to the
downstream service.

---

### Downstream (Fine-grained)

In downstream services, authorization is done in a contextual and explicit
manner, using the **Profile** injected by the gateway.

Conceptual example:

```rust
profile
  .on_tenant(tenant_id)
  .on_account(account_id)
  .with_write_access()
  .with_roles(vec![SystemActor::AccountManager])
  .get_related_account_or_error()?;
```

Each call reduces the set of available capabilities, never expands it.

---

## The Profile

The Profile represents the complete identity context at the time of the request
and acts as an **active authorization object**.

It can contain:

* Identity
* Related tenants and accounts
* Roles
* Scopes and access levels

The Profile exposes primitives such as `on_tenant`, `on_account`, and
`with_roles`, which enable deterministic and auditable decisions.

---

## Formal Classification

* RBAC: used partially and declaratively at the gateway
* ABAC: present in explicit use of attributes
* FBAC: dominant model, contextual and resource-oriented

---

## Auditing and Observability

Each authorization decision can be treated as a logical event containing:

* Resource
* Action
* Applied context
* Decision result

This model is compatible with distributed observability and deterministic
auditing.

---

## Model Evolution

The current model allows incremental evolution to:

* Declarative policy DSL
* Decision caching
* Integration with external engines (optional)

---

## Conclusion

Mycelium implements a modern authorization model, combining declarative controls
at the gateway with contextual decisions close to the resource. This design
favors clarity, security, and scalability without introducing unnecessary
dependencies.
