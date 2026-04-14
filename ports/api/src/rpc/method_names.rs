// Discovery
pub const RPC_DISCOVER: &str = "rpc.discover";

// Managers
pub const MANAGERS_ACCOUNTS_CREATE_SYSTEM_ACCOUNT: &str =
    "managers.accounts.createSystemAccount";
pub const MANAGERS_GUEST_ROLES_CREATE_SYSTEM_ROLES: &str =
    "managers.guestRoles.createSystemRoles";
pub const MANAGERS_TENANTS_CREATE: &str = "managers.tenants.create";
pub const MANAGERS_TENANTS_LIST: &str = "managers.tenants.list";
pub const MANAGERS_TENANTS_DELETE: &str = "managers.tenants.delete";
pub const MANAGERS_TENANTS_INCLUDE_TENANT_OWNER: &str =
    "managers.tenants.includeTenantOwner";
pub const MANAGERS_TENANTS_EXCLUDE_TENANT_OWNER: &str =
    "managers.tenants.excludeTenantOwner";

// Account manager
pub const ACCOUNT_MANAGER_GUESTS_GUEST_TO_CHILDREN_ACCOUNT: &str =
    "accountManager.guests.guestToChildrenAccount";
pub const ACCOUNT_MANAGER_GUEST_ROLES_LIST_GUEST_ROLES: &str =
    "accountManager.guestRoles.listGuestRoles";
pub const ACCOUNT_MANAGER_GUEST_ROLES_FETCH_GUEST_ROLE_DETAILS: &str =
    "accountManager.guestRoles.fetchGuestRoleDetails";

// Gateway manager
pub const GATEWAY_MANAGER_ROUTES_LIST: &str = "gatewayManager.routes.list";
pub const GATEWAY_MANAGER_SERVICES_LIST: &str = "gatewayManager.services.list";
pub const GATEWAY_MANAGER_TOOLS_LIST: &str = "gatewayManager.tools.list";

// Beginners
pub const BEGINNERS_ACCOUNTS_CREATE: &str = "beginners.accounts.create";
pub const BEGINNERS_ACCOUNTS_GET: &str = "beginners.accounts.get";
pub const BEGINNERS_ACCOUNTS_UPDATE_NAME: &str =
    "beginners.accounts.updateName";
pub const BEGINNERS_ACCOUNTS_DELETE: &str = "beginners.accounts.delete";
pub const BEGINNERS_GUESTS_ACCEPT_INVITATION: &str =
    "beginners.guests.acceptInvitation";
pub const BEGINNERS_META_CREATE: &str = "beginners.meta.create";
pub const BEGINNERS_META_UPDATE: &str = "beginners.meta.update";
pub const BEGINNERS_META_DELETE: &str = "beginners.meta.delete";
pub const BEGINNERS_PROFILE_GET: &str = "beginners.profile.get";
pub const BEGINNERS_TENANTS_GET_PUBLIC_INFO: &str =
    "beginners.tenants.getPublicInfo";
pub const BEGINNERS_TOKENS_CREATE: &str = "beginners.tokens.create";
pub const BEGINNERS_TOKENS_DELETE: &str = "beginners.tokens.delete";
pub const BEGINNERS_TOKENS_LIST: &str = "beginners.tokens.list";
pub const BEGINNERS_TOKENS_REVOKE: &str = "beginners.tokens.revoke";
pub const BEGINNERS_USERS: &str = "beginners.users.create";
pub const BEGINNERS_USERS_CHECK_TOKEN_AND_ACTIVATE_USER: &str =
    "beginners.users.checkTokenAndActivateUser";
pub const BEGINNERS_USERS_START_PASSWORD_REDEFINITION: &str =
    "beginners.users.startPasswordRedefinition";
pub const BEGINNERS_USERS_CHECK_TOKEN_AND_RESET_PASSWORD: &str =
    "beginners.users.checkTokenAndResetPassword";
pub const BEGINNERS_USERS_CHECK_EMAIL_PASSWORD_VALIDITY: &str =
    "beginners.users.checkEmailPasswordValidity";
pub const BEGINNERS_USERS_TOTP_START_ACTIVATION: &str =
    "beginners.users.totpStartActivation";
pub const BEGINNERS_USERS_TOTP_FINISH_ACTIVATION: &str =
    "beginners.users.totpFinishActivation";
pub const BEGINNERS_USERS_TOTP_CHECK_TOKEN: &str =
    "beginners.users.totpCheckToken";
pub const BEGINNERS_USERS_TOTP_DISABLE: &str = "beginners.users.totpDisable";

// Guest manager
pub const GUEST_MANAGER_GUEST_ROLES_CREATE: &str =
    "guestManager.guestRoles.create";
pub const GUEST_MANAGER_GUEST_ROLES_LIST: &str = "guestManager.guestRoles.list";
pub const GUEST_MANAGER_GUEST_ROLES_DELETE: &str =
    "guestManager.guestRoles.delete";
pub const GUEST_MANAGER_GUEST_ROLES_UPDATE_NAME_AND_DESCRIPTION: &str =
    "guestManager.guestRoles.updateNameAndDescription";
pub const GUEST_MANAGER_GUEST_ROLES_UPDATE_PERMISSION: &str =
    "guestManager.guestRoles.updatePermission";
pub const GUEST_MANAGER_GUEST_ROLES_INSERT_ROLE_CHILD: &str =
    "guestManager.guestRoles.insertRoleChild";
pub const GUEST_MANAGER_GUEST_ROLES_REMOVE_ROLE_CHILD: &str =
    "guestManager.guestRoles.removeRoleChild";

// System manager
pub const SYSTEM_MANAGER_ERROR_CODES_CREATE: &str =
    "systemManager.errorCodes.create";
pub const SYSTEM_MANAGER_ERROR_CODES_LIST: &str =
    "systemManager.errorCodes.list";
pub const SYSTEM_MANAGER_ERROR_CODES_GET: &str = "systemManager.errorCodes.get";
pub const SYSTEM_MANAGER_ERROR_CODES_UPDATE_MESSAGE_AND_DETAILS: &str =
    "systemManager.errorCodes.updateMessageAndDetails";
pub const SYSTEM_MANAGER_ERROR_CODES_DELETE: &str =
    "systemManager.errorCodes.delete";
pub const SYSTEM_MANAGER_WEBHOOKS_CREATE: &str =
    "systemManager.webhooks.create";
pub const SYSTEM_MANAGER_WEBHOOKS_LIST: &str = "systemManager.webhooks.list";
pub const SYSTEM_MANAGER_WEBHOOKS_UPDATE: &str =
    "systemManager.webhooks.update";
pub const SYSTEM_MANAGER_WEBHOOKS_DELETE: &str =
    "systemManager.webhooks.delete";

// Subscriptions manager
pub const SUBSCRIPTIONS_MANAGER_ACCOUNTS_CREATE_SUBSCRIPTION_ACCOUNT: &str =
    "subscriptionsManager.accounts.createSubscriptionAccount";
pub const SUBSCRIPTIONS_MANAGER_ACCOUNTS_CREATE_ROLE_ASSOCIATED_ACCOUNT: &str =
    "subscriptionsManager.accounts.createRoleAssociatedAccount";
pub const SUBSCRIPTIONS_MANAGER_ACCOUNTS_LIST: &str =
    "subscriptionsManager.accounts.list";
pub const SUBSCRIPTIONS_MANAGER_ACCOUNTS_GET: &str =
    "subscriptionsManager.accounts.get";
pub const SUBSCRIPTIONS_MANAGER_ACCOUNTS_UPDATE_NAME_AND_FLAGS: &str =
    "subscriptionsManager.accounts.updateNameAndFlags";
pub const SUBSCRIPTIONS_MANAGER_ACCOUNTS_PROPAGATE_SUBSCRIPTION_ACCOUNT: &str =
    "subscriptionsManager.accounts.propagateSubscriptionAccount";
pub const SUBSCRIPTIONS_MANAGER_GUESTS_LIST_LICENSED_ACCOUNTS_OF_EMAIL: &str =
    "subscriptionsManager.guests.listLicensedAccountsOfEmail";
pub const SUBSCRIPTIONS_MANAGER_GUESTS_GUEST_USER_TO_SUBSCRIPTION_ACCOUNT:
    &str = "subscriptionsManager.guests.guestUserToSubscriptionAccount";
pub const SUBSCRIPTIONS_MANAGER_GUESTS_UPDATE_FLAGS_FROM_SUBSCRIPTION_ACCOUNT: &str =
    "subscriptionsManager.guests.updateFlagsFromSubscriptionAccount";
pub const SUBSCRIPTIONS_MANAGER_GUESTS_REVOKE_USER_GUEST_TO_SUBSCRIPTION_ACCOUNT: &str =
    "subscriptionsManager.guests.revokeUserGuestToSubscriptionAccount";
pub const SUBSCRIPTIONS_MANAGER_GUESTS_LIST_GUEST_ON_SUBSCRIPTION_ACCOUNT:
    &str = "subscriptionsManager.guests.listGuestOnSubscriptionAccount";
pub const SUBSCRIPTIONS_MANAGER_GUEST_ROLES_LIST: &str =
    "subscriptionsManager.guestRoles.list";
pub const SUBSCRIPTIONS_MANAGER_GUEST_ROLES_GET: &str =
    "subscriptionsManager.guestRoles.get";
pub const SUBSCRIPTIONS_MANAGER_TAGS_CREATE: &str =
    "subscriptionsManager.tags.create";
pub const SUBSCRIPTIONS_MANAGER_TAGS_UPDATE: &str =
    "subscriptionsManager.tags.update";
pub const SUBSCRIPTIONS_MANAGER_TAGS_DELETE: &str =
    "subscriptionsManager.tags.delete";

// Tenant manager
pub const TENANT_MANAGER_ACCOUNTS_CREATE_SUBSCRIPTION_MANAGER_ACCOUNT: &str =
    "tenantManager.accounts.createSubscriptionManagerAccount";
pub const TENANT_MANAGER_ACCOUNTS_DELETE_SUBSCRIPTION_ACCOUNT: &str =
    "tenantManager.accounts.deleteSubscriptionAccount";
pub const TENANT_MANAGER_GUESTS_GUEST_USER_TO_SUBSCRIPTION_MANAGER_ACCOUNT:
    &str = "tenantManager.guests.guestUserToSubscriptionManagerAccount";
pub const TENANT_MANAGER_GUESTS_REVOKE_USER_GUEST_TO_SUBSCRIPTION_MANAGER_ACCOUNT: &str =
    "tenantManager.guests.revokeUserGuestToSubscriptionManagerAccount";
pub const TENANT_MANAGER_TAGS_CREATE: &str = "tenantManager.tags.create";
pub const TENANT_MANAGER_TAGS_UPDATE: &str = "tenantManager.tags.update";
pub const TENANT_MANAGER_TAGS_DELETE: &str = "tenantManager.tags.delete";
pub const TENANT_MANAGER_TENANT_GET: &str = "tenantManager.tenant.get";

// Tenant owner
pub const TENANT_OWNER_ACCOUNTS_CREATE_MANAGEMENT_ACCOUNT: &str =
    "tenantOwner.accounts.createManagementAccount";
pub const TENANT_OWNER_ACCOUNTS_DELETE_TENANT_MANAGER_ACCOUNT: &str =
    "tenantOwner.accounts.deleteTenantManagerAccount";
pub const TENANT_OWNER_META_CREATE: &str = "tenantOwner.meta.create";
pub const TENANT_OWNER_META_DELETE: &str = "tenantOwner.meta.delete";
pub const TENANT_OWNER_OWNER_GUEST: &str = "tenantOwner.owner.guest";
pub const TENANT_OWNER_OWNER_REVOKE: &str = "tenantOwner.owner.revoke";
pub const TENANT_OWNER_TENANT_UPDATE_AND_DESCRIPTION: &str =
    "tenantOwner.tenant.updateNameAndDescription";
pub const TENANT_OWNER_TENANT_UPDATE_ARCHIVING_STATUS: &str =
    "tenantOwner.tenant.updateArchivingStatus";
pub const TENANT_OWNER_TENANT_UPDATE_TRASHING_STATUS: &str =
    "tenantOwner.tenant.updateTrashingStatus";
pub const TENANT_OWNER_TENANT_UPDATE_VERIFYING_STATUS: &str =
    "tenantOwner.tenant.updateVerifyingStatus";

// Users manager
pub const USER_MANAGER_ACCOUNT_APPROVE: &str = "userManager.account.approve";
pub const USER_MANAGER_ACCOUNT_DISAPPROVE: &str =
    "userManager.account.disapprove";
pub const USER_MANAGER_ACCOUNT_ACTIVATE: &str = "userManager.account.activate";
pub const USER_MANAGER_ACCOUNT_DEACTIVATE: &str =
    "userManager.account.deactivate";
pub const USER_MANAGER_ACCOUNT_ARCHIVE: &str = "userManager.account.archive";
pub const USER_MANAGER_ACCOUNT_UNARCHIVE: &str =
    "userManager.account.unarchive";

// Service
pub const SERVICE_LIST_DISCOVERABLE_SERVICES: &str =
    "service.listDiscoverableServices";

// Staff
pub const STAFF_ACCOUNTS_UPGRADE_PRIVILEGES: &str =
    "staff.accounts.upgradePrivileges";
pub const STAFF_ACCOUNTS_DOWNGRADE_PRIVILEGES: &str =
    "staff.accounts.downgradePrivileges";
