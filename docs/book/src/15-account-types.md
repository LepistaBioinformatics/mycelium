# Account Types and Roles

Mycelium uses a layered identity model:

- An **account** is the unit of identity.
- An **account type** describes the purpose and scope of that account.
- A **role** (`SystemActor`) determines which administrative operations the account can perform.
- A **guest relationship** links a `User` account to a tenant-scoped account, granting it
  contextual access with a specific permission level.

---

## Quick reference — which account type do I need?

| Scenario | Account type |
|---|---|
| Human end user logging in | `User` |
| Platform operator / superadmin | `Staff` |
| Delegated platform administrator | `Manager` |
| Service, bot, or non-human entity within a tenant | `Subscription` |
| Service account with a built-in administrative role inside a tenant | `RoleAssociated` |
| Delegated tenant administrator | `TenantManager` |
| Internal system-level actor | `ActorAssociated` |

---

## Account types

Every account in Mycelium has an `accountType` field. Its value determines what the account
can do and which management operations apply to it.

### `User`

```json
"accountType": "user"
```

A personal account belonging to a human user. The default type for end users who log in via
email/password, magic link, OAuth, or any configured external IdP (e.g. Telegram).

- Has no administrative privileges by default.
- Can belong to multiple tenants simultaneously by being *guested* into tenant-scoped
  `Subscription` accounts (see [Tenant membership](#tenant-membership-guest-relationships) below).
- Managed by the `UsersManager` role (approval, activation, archival).

### `Staff`

```json
"accountType": "staff"
```

A platform-level administrative account for operators who control the entire Mycelium instance.
Staff accounts can create tenants, manage platform-wide guest roles, and upgrade or downgrade
other accounts. They are **not** tenant-scoped — they act across all tenants.

The first `Staff` account is created via the CLI (`myc-cli accounts create-seed-account`).
See [CLI Reference](./18-cli.md).

### `Manager`

```json
"accountType": "manager"
```

Similar to `Staff` but intended for delegated platform management. Managers can create system
accounts and manage tenant membership at the platform level without holding the highest-privilege
`Staff` designation. Suitable for operations teams that need broad but not full superadmin access.

### `Subscription`

```json
"accountType": { "subscription": { "tenantId": "<uuid>" } }
```

A tenant-scoped account representing a service, bot, or non-human entity (e.g. an external
application, an automated pipeline, an integration). Created by a `TenantManager` within a
specific tenant.

`User` accounts join a tenant by being *guested* into a `Subscription` account with a
specific guest role and permission level (see below). The subscription account is the anchor
for all tenant-scoped permissions.

Managed by the `SubscriptionsManager` role (invite guests, update name and flags).

### `RoleAssociated`

```json
"accountType": {
  "roleAssociated": {
    "tenantId": "<uuid>",
    "roleName": "subscriptions-manager",
    "readRoleId": "<uuid>",
    "writeRoleId": "<uuid>"
  }
}
```

A `Subscription`-like account that is pinned to a specific named guest role. Used to create
service accounts that carry a built-in administrative role inside a tenant — the canonical
example is a **Subscription Manager** account, which is a `RoleAssociated` account bound to
the `SubscriptionsManager` system actor role.

This account type is created automatically when calling the
`tenant-manager/create-subscription-manager-account` endpoint. It is a system-managed type;
you rarely create it manually.

### `TenantManager`

```json
"accountType": { "tenantManager": { "tenantId": "<uuid>" } }
```

A management account scoped to a specific tenant. Created by tenant owners (`TenantOwner` role)
to delegate tenant-level administrative tasks. Tenant managers can create and delete subscription
accounts, manage tenant tags, and invite subscription managers within their tenant.

### `ActorAssociated`

```json
"accountType": { "actorAssociated": { "actor": "<SystemActor>" } }
```

An internal account bound to a specific `SystemActor` role. Used by Mycelium itself to
represent system-level actors that need a persistent identity (e.g. for audit trails).
You do not create these manually — the platform provisions them as needed.

---

## Tenant membership (guest relationships)

An account type alone does not grant access to a tenant's resources. Access is established
through **guest relationships**:

```
User account
  └── guested into → Subscription account (tenant-scoped)
                          └── with a GuestRole + Permission level
                                  └── grants access to downstream routes
```

### How a user joins a tenant

1. A `TenantManager` or `SubscriptionsManager` creates a `Subscription` account for the tenant.
2. The subscription manager invites a user by email (`guest_user_to_subscription_account`).
3. The user receives an invitation email and accepts it.
4. The user's `User` account is now a *guest* of the subscription account, holding a
   `GuestRole` at a specific permission level (`Read` or `Write`).
5. When the user sends a request with `x-mycelium-tenant-id`, Mycelium resolves their profile
   to include the tenant-scoped permissions from all subscription accounts they are guested into.

### Guesting to child accounts

If a `Subscription` account has child accounts (set up via `RoleAssociated` accounts), an
`AccountManager` can further delegate access: inviting a user into a child account
(`guest_to_children_account`) so the user operates only within the narrower scope of that
child account rather than the full subscription account.

### Permission levels within guest roles

| Permission | What it allows |
|---|---|
| `Read` | Read-only access within the scope of the role |
| `Write` | Read and write access within the scope of the role |

Downstream routes declare their required permission level in the route config:

```toml
[group]
protectedByRoles = [{ slug = "editor", permission = "write" }]
```

Users whose guest role carries only `Read` permission are rejected with `403` on write routes.

---

## Administrative roles (SystemActor)

Every administrative route and JSON-RPC namespace is guarded by a role. In REST, the role
appears as the path segment immediately after `/_adm/`.

| Role (`SystemActor`) | URL path | Typical scope |
|---|---|---|
| `Beginners` | `/_adm/beginners/` | Any authenticated user — own profile, tokens, invitations |
| `SubscriptionsManager` | `/_adm/subscriptions-manager/` | Invite guests, manage subscription accounts within a tenant |
| `UsersManager` | `/_adm/users-manager/` | Approve, activate, archive, suspend user accounts (platform) |
| `AccountManager` | `/_adm/account-manager/` | Invite guests to child accounts |
| `GuestsManager` | `/_adm/guests-manager/` | Create, update, delete guest roles |
| `GatewayManager` | `/_adm/gateway-manager/` | Read-only inspection of routes and services |
| `SystemManager` | `/_adm/system-manager/` | Error codes, outbound webhooks (platform-wide) |
| `TenantOwner` | `/_adm/tenant-owner/` | Ownership-level operations on a specific tenant |
| `TenantManager` | `/_adm/tenant-manager/` | Delegated management within a specific tenant |

`Staff` and `Manager` accounts additionally access the `managers.*` JSON-RPC namespace and
REST paths under `/_adm/managers/`, which sit above the per-tenant role hierarchy.

`Beginners` is not an administrative role — it is the namespace for self-service operations
any authenticated user may perform.

---

## Role hierarchy

```
Staff / Manager  (platform-wide)
  └── create tenants, create system accounts, manage platform guest roles
        │
        ├── TenantOwner  (per tenant)
        │     ├── create / delete TenantManager accounts
        │     ├── manage tenant metadata, archiving, verification
        │     └── configure external IdPs (e.g. Telegram bot token)
        │
        ├── TenantManager  (per tenant)
        │     ├── create / delete Subscription accounts
        │     ├── create SubscriptionManager (RoleAssociated) accounts
        │     └── manage tenant tags
        │
        ├── SubscriptionsManager  (per tenant, via RoleAssociated account)
        │     ├── invite guests to Subscription accounts
        │     └── create RoleAssociated accounts
        │
        ├── GuestsManager  (platform or tenant)
        │     └── define guest roles and their permissions
        │
        ├── UsersManager  (platform)
        │     └── approve / activate / archive User accounts
        │
        ├── AccountManager  (per tenant)
        │     └── invite guests to child accounts
        │
        └── GatewayManager  (platform)
              └── read-only inspection of routes and services
```

---

## How roles are enforced

When a request arrives at an admin route (e.g. `POST /_adm/tenant-owner/...`), Mycelium:

1. Validates the token (JWT or connection string).
2. Resolves the caller's profile, which includes their account type, tenant memberships,
   and guest roles.
3. Checks whether the resolved profile satisfies the required `SystemActor` for that route.
4. For tenant-scoped operations, also checks the `x-mycelium-tenant-id` header.
5. Returns `403` if the role or permission is missing; forwards to the use-case layer on match.

---

## Seed account

The first `Staff` account in a fresh installation is created from the CLI before any user
can log in:

```bash
myc-cli accounts create-seed-account \
  --name "Platform Admin" \
  --email admin@example.com
```

See [CLI Reference](./18-cli.md) for the full command reference.
