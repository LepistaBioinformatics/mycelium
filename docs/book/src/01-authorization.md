# Authorization Model

This page explains how Mycelium decides whether a request is allowed to proceed.

---

## The short version

Mycelium uses a **two-layer** approach:

1. **Gateway layer** — coarse checks at the route level ("is this user logged in? do they have
   the right role?"). If the check fails, the request is rejected before reaching your service.

2. **Downstream layer** — fine-grained checks inside your service, using the identity that
   Mycelium injects ("does this user have write access to *this specific resource*?").

Think of it like a building: Mycelium is the security desk at the entrance (checks your ID
and badge before letting you through the door). Your service is the room inside — it gets
to decide what the person can do once they're in.

---

## The gateway layer

When a request arrives, Mycelium matches it against the configured route. Each route belongs
to a **security group** that defines the minimum requirements:

| Security group | What Mycelium checks |
|---|---|
| `public` | Nothing — anyone can pass |
| `authenticated` | Valid JWT or connection string |
| `protected` | Valid token + resolved profile |
| `protectedByRoles` | Valid token + user has one of the listed roles |

If the check passes, Mycelium forwards the request to your service and injects the user's
identity as HTTP headers.

---

## What gets injected

Depending on the security group, your service receives:

| Header | When injected | Contains |
|---|---|---|
| `x-mycelium-email` | `authenticated` or higher | The authenticated user's email |
| `x-mycelium-profile` | `protected` or higher | Full identity context (see below) |

The profile is a compressed JSON object carrying: account ID, tenant memberships, roles, and
access scopes. Your service reads it and can make resource-level decisions without querying
the gateway or doing its own authentication.

---

## The downstream layer

Once a request is inside your service, you use the profile to decide what the user can do
with a specific resource. The profile exposes a fluent API for narrowing the access context:

```
profile
  .on_tenant(tenant_id)     # focus on this tenant
  .on_account(account_id)   # focus on this account
  .with_write_access()      # must have write permission
  .with_roles(["manager"])  # must have manager role
  .get_related_account_or_error()  # returns error if no match
```

Each step narrows — never expands — the set of permissions. If any step finds no match, the
chain returns an error and you return 403 to your caller.

This design means:
- Access decisions are explicit and auditable.
- No implicit "superuser" paths that bypass checks.
- Your service never needs to call Mycelium again to validate permissions.

---

## Reference

### Formal classification

Mycelium's model spans three standard paradigms:

- **RBAC** (Role-Based Access Control) — used declaratively at the gateway level (security groups with role lists).
- **ABAC** (Attribute-Based Access Control) — the profile carries attributes (tenant, account, scope) used in downstream decisions.
- **FBAC** (Feature-Based Access Control) — the dominant model; access decisions are made close to the resource using the full contextual chain.

### Design principles

- Authentication, identity enrichment, and authorization are strictly separated.
- Capabilities are progressively reduced — the chain never grants more than the token allows.
- No global policies that silently override explicit checks.
- Each authorization decision is a discrete, loggable event (resource, action, context, outcome).
