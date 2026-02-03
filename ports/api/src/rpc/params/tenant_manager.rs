use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionManagerAccountParams {
    pub tenant_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSubscriptionAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
}

// ---------------------------------------------------------------------------
// Guests
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserToSubscriptionManagerAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub email: String,
    #[schemars(description = "0 = Read, 1 = Write")]
    pub permission: i32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeUserGuestToSubscriptionManagerAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub role_id: Uuid,
    pub email: String,
}

// ---------------------------------------------------------------------------
// Tags (tenant-scoped, no account_id)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantManagerRegisterTagParams {
    pub tenant_id: Uuid,
    pub value: String,
    pub meta: Option<HashMap<String, String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantManagerUpdateTagParams {
    pub tenant_id: Uuid,
    pub tag_id: Uuid,
    pub value: String,
    pub meta: Option<HashMap<String, String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantManagerDeleteTagParams {
    pub tenant_id: Uuid,
    pub tag_id: Uuid,
}

// ---------------------------------------------------------------------------
// Tenant
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetTenantDetailsParams {
    pub tenant_id: Uuid,
}
