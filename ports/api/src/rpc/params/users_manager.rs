use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Account
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserManagerAccountIdParams {
    pub account_id: Uuid,
}
