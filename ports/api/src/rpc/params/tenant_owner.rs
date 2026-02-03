use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateManagementAccountParams {
    pub tenant_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTenantManagerAccountParams {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantMetaParams {
    pub tenant_id: Uuid,
    #[schemars(
        description = "e.g. federal_revenue_register, locale, legal_name"
    )]
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTenantMetaParams {
    pub tenant_id: Uuid,
    pub key: String,
}

// ---------------------------------------------------------------------------
// Owner
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestTenantOwnerParams {
    pub tenant_id: Uuid,
    pub email: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeTenantOwnerParams {
    pub tenant_id: Uuid,
    pub email: String,
}

// ---------------------------------------------------------------------------
// Tenant
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantNameAndDescriptionParams {
    pub tenant_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantArchivingStatusParams {
    pub tenant_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantTrashingStatusParams {
    pub tenant_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantVerifyingStatusParams {
    pub tenant_id: Uuid,
}
