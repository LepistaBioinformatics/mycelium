//! JSON-RPC param DTOs for beginners scope (beginners.accounts.*).

use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultAccountParams {
    #[schemars(description = "Account name")]
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOwnAccountNameParams {
    #[schemars(description = "Account ID (must match the authenticated user's account)")]
    pub account_id: Uuid,
    #[schemars(description = "New account name")]
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMyAccountParams {
    #[schemars(description = "Account ID (must match the authenticated user's account)")]
    pub account_id: Uuid,
}
