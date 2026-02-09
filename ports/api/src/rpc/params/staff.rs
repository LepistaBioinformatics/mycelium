use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeAccountPrivilegesParams {
    pub account_id: Uuid,
    #[schemars(description = "Target type: Staff or Manager")]
    pub to: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DowngradeAccountPrivilegesParams {
    pub account_id: Uuid,
    #[schemars(description = "Target type: Manager or User")]
    pub to: String,
}
