# JSON-RPC Interface

Mycelium exposes all administrative operations through a **JSON-RPC 2.0** interface at
`POST /_adm/rpc`. This endpoint accepts both single requests and batched arrays of requests.

The JSON-RPC interface mirrors the REST admin API — every operation available through the REST
routes is also available as a JSON-RPC method. Prefer JSON-RPC when building programmatic clients,
automation scripts, or AI agent integrations.

---

## Transport

```
POST /_adm/rpc
Authorization: Bearer <jwt>          # or x-mycelium-connection-string
Content-Type: application/json
```

Single request:

```json
{
  "jsonrpc": "2.0",
  "method": "beginners.profile.get",
  "params": {},
  "id": 1
}
```

Batch request (array):

```json
[
  { "jsonrpc": "2.0", "method": "beginners.profile.get", "params": {}, "id": 1 },
  { "jsonrpc": "2.0", "method": "gatewayManager.routes.list", "params": {}, "id": 2 }
]
```

---

## Discovery

### `rpc.discover`

Returns the OpenRPC specification for this server — the full list of methods, their parameter
schemas, and result schemas.

```json
{ "jsonrpc": "2.0", "method": "rpc.discover", "params": {}, "id": 1 }
```

---

## Method Reference

Methods are organized by namespace. The namespace corresponds to the role required to call them
(see [Account Types and Roles](./15-account-types.md)).

### `managers` — Platform-wide operations

Requires: **staff** or **manager** role.

| Method | Description |
|---|---|
| `managers.accounts.createSystemAccount` | Create a system-level account |
| `managers.guestRoles.createSystemRoles` | Create platform-wide guest roles |
| `managers.tenants.create` | Create a new tenant |
| `managers.tenants.list` | List all tenants |
| `managers.tenants.delete` | Delete a tenant |
| `managers.tenants.includeTenantOwner` | Assign a tenant owner to a tenant |
| `managers.tenants.excludeTenantOwner` | Remove a tenant owner from a tenant |

---

### `accountManager` — Account manager operations

Requires: **accounts-manager** role.

| Method | Description |
|---|---|
| `accountManager.guests.guestToChildrenAccount` | Invite a user as a guest to a child account |
| `accountManager.guestRoles.listGuestRoles` | List available guest roles |
| `accountManager.guestRoles.fetchGuestRoleDetails` | Get details of a specific guest role |

---

### `gatewayManager` — Gateway inspection

Requires: **gateway-manager** role.

| Method | Description |
|---|---|
| `gatewayManager.routes.list` | List all registered routes |
| `gatewayManager.services.list` | List all registered downstream services |
| `gatewayManager.tools.list` | List all discoverable tools exposed to AI agents |

---

### `beginners` — Self-service / user operations

Requires: **authenticated** (any logged-in user). No admin role needed.

**Accounts**

| Method | Description |
|---|---|
| `beginners.accounts.create` | Create a personal account |
| `beginners.accounts.get` | Get own account details |
| `beginners.accounts.updateName` | Update own display name |
| `beginners.accounts.delete` | Delete own account |

**Profile**

| Method | Description |
|---|---|
| `beginners.profile.get` | Get own resolved profile (tenants, roles, access scopes) |

**Tenants**

| Method | Description |
|---|---|
| `beginners.tenants.getPublicInfo` | Get public metadata for a tenant |

**Guests**

| Method | Description |
|---|---|
| `beginners.guests.acceptInvitation` | Accept a guest invitation to a tenant |

**Tokens**

| Method | Description |
|---|---|
| `beginners.tokens.create` | Create a connection string token |
| `beginners.tokens.list` | List own active tokens |
| `beginners.tokens.revoke` | Revoke a specific token |
| `beginners.tokens.delete` | Delete a token |

**Metadata**

| Method | Description |
|---|---|
| `beginners.meta.create` | Create account metadata entry |
| `beginners.meta.update` | Update account metadata entry |
| `beginners.meta.delete` | Delete account metadata entry |

**Users and authentication**

| Method | Description |
|---|---|
| `beginners.users.create` | Register a new user credential |
| `beginners.users.checkTokenAndActivateUser` | Activate user via email token |
| `beginners.users.startPasswordRedefinition` | Start password reset flow |
| `beginners.users.checkTokenAndResetPassword` | Complete password reset |
| `beginners.users.checkEmailPasswordValidity` | Validate credentials |
| `beginners.users.totpStartActivation` | Begin TOTP 2FA setup |
| `beginners.users.totpFinishActivation` | Complete TOTP 2FA setup |
| `beginners.users.totpCheckToken` | Verify a TOTP code |
| `beginners.users.totpDisable` | Disable TOTP 2FA |

---

### `guestManager` — Guest role management

Requires: **guests-manager** role.

| Method | Description |
|---|---|
| `guestManager.guestRoles.create` | Create a guest role |
| `guestManager.guestRoles.list` | List guest roles |
| `guestManager.guestRoles.delete` | Delete a guest role |
| `guestManager.guestRoles.updateNameAndDescription` | Update role name and description |
| `guestManager.guestRoles.updatePermission` | Update role permission level |
| `guestManager.guestRoles.insertRoleChild` | Add a child role to a parent |
| `guestManager.guestRoles.removeRoleChild` | Remove a child role |

---

### `systemManager` — System-level management

Requires: **system-manager** role.

**Error codes**

| Method | Description |
|---|---|
| `systemManager.errorCodes.create` | Create a custom error code |
| `systemManager.errorCodes.list` | List all error codes |
| `systemManager.errorCodes.get` | Get a specific error code |
| `systemManager.errorCodes.updateMessageAndDetails` | Update error code message and details |
| `systemManager.errorCodes.delete` | Delete an error code |

**Webhooks**

| Method | Description |
|---|---|
| `systemManager.webhooks.create` | Register an outbound webhook |
| `systemManager.webhooks.list` | List registered webhooks |
| `systemManager.webhooks.update` | Update a webhook |
| `systemManager.webhooks.delete` | Delete a webhook |

---

### `subscriptionsManager` — Subscription account management

Requires: **subscriptions-manager** role.

**Accounts**

| Method | Description |
|---|---|
| `subscriptionsManager.accounts.createSubscriptionAccount` | Create a subscription account |
| `subscriptionsManager.accounts.createRoleAssociatedAccount` | Create a role-associated account |
| `subscriptionsManager.accounts.list` | List subscription accounts |
| `subscriptionsManager.accounts.get` | Get a specific subscription account |
| `subscriptionsManager.accounts.updateNameAndFlags` | Update account name and flags |
| `subscriptionsManager.accounts.propagateSubscriptionAccount` | Propagate account across tenants |

**Guests**

| Method | Description |
|---|---|
| `subscriptionsManager.guests.listLicensedAccountsOfEmail` | List licensed accounts for an email |
| `subscriptionsManager.guests.guestUserToSubscriptionAccount` | Invite a user to a subscription account |
| `subscriptionsManager.guests.updateFlagsFromSubscriptionAccount` | Update guest flags |
| `subscriptionsManager.guests.revokeUserGuestToSubscriptionAccount` | Revoke a guest invitation |
| `subscriptionsManager.guests.listGuestOnSubscriptionAccount` | List guests on a subscription account |

**Guest roles**

| Method | Description |
|---|---|
| `subscriptionsManager.guestRoles.list` | List guest roles for a subscription |
| `subscriptionsManager.guestRoles.get` | Get a specific guest role |

**Tags**

| Method | Description |
|---|---|
| `subscriptionsManager.tags.create` | Create a tag |
| `subscriptionsManager.tags.update` | Update a tag |
| `subscriptionsManager.tags.delete` | Delete a tag |

---

### `tenantManager` — Tenant internal management

Requires: **tenant-manager** role.

**Accounts**

| Method | Description |
|---|---|
| `tenantManager.accounts.createSubscriptionManagerAccount` | Create a subscription manager account within the tenant |
| `tenantManager.accounts.deleteSubscriptionAccount` | Delete a subscription account |

**Guests**

| Method | Description |
|---|---|
| `tenantManager.guests.guestUserToSubscriptionManagerAccount` | Invite a user as subscription manager |
| `tenantManager.guests.revokeUserGuestToSubscriptionManagerAccount` | Revoke subscription manager invitation |

**Tags**

| Method | Description |
|---|---|
| `tenantManager.tags.create` | Create a tenant tag |
| `tenantManager.tags.update` | Update a tenant tag |
| `tenantManager.tags.delete` | Delete a tenant tag |

**Tenant**

| Method | Description |
|---|---|
| `tenantManager.tenant.get` | Get tenant details |

---

### `tenantOwner` — Tenant ownership operations

Requires: **tenant-owner** role.

**Accounts**

| Method | Description |
|---|---|
| `tenantOwner.accounts.createManagementAccount` | Create a management account for the tenant |
| `tenantOwner.accounts.deleteTenantManagerAccount` | Delete a tenant manager account |

**Metadata**

| Method | Description |
|---|---|
| `tenantOwner.meta.create` | Create tenant metadata |
| `tenantOwner.meta.delete` | Delete tenant metadata |

**Ownership**

| Method | Description |
|---|---|
| `tenantOwner.owner.guest` | Add a co-owner to the tenant |
| `tenantOwner.owner.revoke` | Remove a co-owner from the tenant |

**Tenant**

| Method | Description |
|---|---|
| `tenantOwner.tenant.updateNameAndDescription` | Update tenant name and description |
| `tenantOwner.tenant.updateArchivingStatus` | Archive or unarchive the tenant |
| `tenantOwner.tenant.updateTrashingStatus` | Trash or restore the tenant |
| `tenantOwner.tenant.updateVerifyingStatus` | Mark the tenant as verified or unverified |

---

### `userManager` — User account lifecycle (platform admins)

Requires: **users-manager** role.

| Method | Description |
|---|---|
| `userManager.account.approve` | Approve a user account registration |
| `userManager.account.disapprove` | Disapprove a user account registration |
| `userManager.account.activate` | Re-activate a suspended user account |
| `userManager.account.deactivate` | Suspend a user account |
| `userManager.account.archive` | Archive a user account |
| `userManager.account.unarchive` | Restore an archived user account |

---

### `service` — Service discovery

Requires: authenticated.

| Method | Description |
|---|---|
| `service.listDiscoverableServices` | List all services marked as discoverable for AI agent use |

---

### `staff` — Privilege escalation

Requires: **staff** role.

| Method | Description |
|---|---|
| `staff.accounts.upgradePrivileges` | Upgrade an account to staff privileges |
| `staff.accounts.downgradePrivileges` | Downgrade a staff account to standard privileges |

---

## Error responses

JSON-RPC errors follow the standard format:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid request"
  },
  "id": null
}
```

Standard error codes:

| Code | Meaning |
|---|---|
| `-32700` | Parse error — invalid JSON |
| `-32600` | Invalid request — not a valid JSON-RPC 2.0 object |
| `-32601` | Method not found |
| `-32602` | Invalid params |
| `-32603` | Internal error |

Authentication failures return HTTP 401 before the JSON-RPC layer is reached. Authorization
failures (wrong role) return a JSON-RPC error with code `-32603` and a domain-specific message.
