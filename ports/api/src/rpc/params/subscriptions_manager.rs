use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionAccountParams {
    #[schemars(description = "Tenant ID")]
    pub tenant_id: Uuid,
    #[schemars(description = "Account name")]
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleAssociatedAccountParams {
    #[schemars(description = "Tenant ID")]
    pub tenant_id: Uuid,
    pub account_name: String,
    pub role_name: String,
    pub role_description: String,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAccountsByTypeParams {
    #[schemars(
        description = "Optional tenant ID (required for Subscription, TenantManager, RoleAssociated)"
    )]
    pub tenant_id: Option<Uuid>,
    pub term: Option<String>,
    pub tag_value: Option<String>,
    #[schemars(
        description = "Account type: Staff, Manager, User, Subscription, TenantManager, ActorAssociated, RoleAssociated"
    )]
    pub account_type: Option<String>,
    pub is_owner_active: Option<bool>,
    #[schemars(
        description = "Status: unverified, verified, inactive, archived, deleted"
    )]
    pub status: Option<String>,
    #[schemars(
        description = "Actor for ActorAssociated: gatewayManager, guestsManager, systemManager"
    )]
    pub actor: Option<String>,
    #[schemars(description = "For RoleAssociated")]
    pub role_name: Option<String>,
    pub read_role_id: Option<Uuid>,
    pub write_role_id: Option<Uuid>,
    pub page_size: Option<i32>,
    pub skip: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetAccountDetailsParams {
    pub tenant_id: Option<Uuid>,
    pub account_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountNameAndFlagsParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub is_checked: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_system_account: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PropagateSubscriptionAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
}

// ---------------------------------------------------------------------------
// Guests
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListLicensedAccountsOfEmailParams {
    pub tenant_id: Uuid,
    pub email: String,
    #[schemars(
        description = "Optional roles filter (array of { name, permission? })"
    )]
    pub roles: Option<Vec<RoleParam>>,
    pub was_verified: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleParam {
    pub name: String,
    #[schemars(description = "0 = Read, 1 = Write")]
    pub permission: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserToSubscriptionAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub role_id: Uuid,
    pub email: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFlagsFromSubscriptionAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub role_id: Uuid,
    pub permit_flags: Option<Vec<String>>,
    pub deny_flags: Option<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeUserGuestToSubscriptionAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub role_id: Uuid,
    pub email: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestOnSubscriptionAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub page_size: Option<i32>,
    pub skip: Option<i32>,
}

// ---------------------------------------------------------------------------
// Guest roles
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsManagerListGuestRolesParams {
    pub tenant_id: Option<Uuid>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub system: Option<bool>,
    pub page_size: Option<i32>,
    pub skip: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsManagerFetchGuestRoleDetailsParams {
    pub tenant_id: Option<Uuid>,
    pub id: Uuid,
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterTagParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub value: String,
    pub meta: Option<HashMap<String, String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub tag_id: Uuid,
    pub value: String,
    pub meta: Option<HashMap<String, String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTagParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub tag_id: Uuid,
}
