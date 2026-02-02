use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSystemAccountParams {
    #[schemars(description = "Account name")]
    pub name: String,
    #[schemars(
        description = "System actor type: gatewayManager, guestsManager, systemManager"
    )]
    pub actor: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantParams {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListTenantParams {
    pub name: Option<String>,
    pub owner: Option<Uuid>,
    #[schemars(description = "key=value")]
    pub metadata: Option<String>,
    #[schemars(description = "key=value")]
    pub tag: Option<String>,
    pub page_size: Option<i32>,
    pub skip: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTenantParams {
    pub id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IncludeTenantOwnerParams {
    #[schemars(description = "Tenant ID")]
    pub id: Uuid,
    pub owner_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExcludeTenantOwnerParams {
    #[schemars(description = "Tenant ID")]
    pub id: Uuid,
    pub owner_id: Uuid,
}
