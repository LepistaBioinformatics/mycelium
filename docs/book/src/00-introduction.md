# Introduction

**Mycelium API Gateway** sits in front of your backend services and handles authentication,
authorization, and routing — so your services don't have to.

---

## Who is this for?

This documentation is written for three types of readers:

**Operator** — You're deploying Mycelium for an organization. You'll configure tenants, users,
and which backend services are reachable through the gateway. Start with
[Installation](./02-installation.md) and [Quick Start](./03-quick-start.md).

**Backend developer** — You're building a service that sits behind Mycelium. The gateway
will handle authentication and then inject the user's identity into your requests via
headers. Start with [Downstream APIs](./06-downstream-apis.md).

**End user** — You're using a product built on Mycelium. You'll authenticate via email magic
link, or through an alternative identity provider like Telegram. Your experience depends on
how the operator has configured their instance.

---

## What does Mycelium do?

```
Your users
    ↓
Mycelium API Gateway   ← handles login, token validation, and routing decisions
    ↓
Your backend services  ← receive authenticated requests with user identity in headers
```

When a request arrives, Mycelium checks:

1. **Who are you?** (authentication — via magic link, OAuth2, Telegram, etc.)
2. **Are you allowed here?** (coarse authorization — role checks at the route level)
3. **Where should this go?** (routing — forwards to the right downstream service)

Your backend service receives the request with the user's identity already resolved and
injected as an HTTP header (`x-mycelium-profile`). It can then make fine-grained decisions
without doing its own authentication.

---

## Key concepts

**Tenant** — A company or organization within your Mycelium installation. Users belong to
tenants, and access controls are applied per tenant.

**Account** — The unit of identity in Mycelium. There are several account types: `User`
(human end users), `Staff` / `Manager` (platform administrators), `Subscription`
(tenant-scoped services or bots), `TenantManager` (delegated tenant admins), and others.
A `User` account joins a tenant by being *guested* into a `Subscription` account with a
specific role and permission level. See [Account Types and Roles](./15-account-types.md)
for the full model.

**Profile** — A snapshot of the authenticated user's identity at the time of the request: their
account, tenant memberships, roles, and access levels. Injected into every downstream request.

**Security group** — A label on each route that tells Mycelium what level of authentication is
required. Options range from `public` (anyone) to `protectedByRoles` (specific roles only).

---

## How authentication works

Mycelium ships with built-in email + magic-link login. No passwords required — the user enters
their email, receives a one-time link, and gets a JWT token.

You can also connect external OAuth2 providers (Google, Microsoft, Auth0) or alternative identity
providers like Telegram. See [Alternative Identity Providers](./10-alternative-idps.md) for details.

---

## Next steps

- [Installation](./02-installation.md) — Install the gateway binary or Docker image
- [Quick Start](./03-quick-start.md) — Get a running instance in minutes
- [Configuration](./04-configuration.md) — Full configuration reference
- [Downstream APIs](./06-downstream-apis.md) — Register your backend services
- [Authorization Model](./01-authorization.md) — Understand how access decisions are made
