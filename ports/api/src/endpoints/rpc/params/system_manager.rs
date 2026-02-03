use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterErrorCodeParams {
    pub prefix: String,
    pub message: String,
    #[schemars(description = "Optional details")]
    pub details: Option<String>,
    pub is_internal: bool,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListErrorCodesParams {
    pub prefix: Option<String>,
    pub code: Option<i32>,
    pub is_internal: Option<bool>,
    pub page_size: Option<i32>,
    pub skip: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetErrorCodeParams {
    pub prefix: String,
    pub code: i32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateErrorCodeMessageAndDetailsParams {
    pub prefix: String,
    pub code: i32,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteErrorCodeParams {
    pub prefix: String,
    pub code: i32,
}

// ---------------------------------------------------------------------------
// Webhooks (trigger/method/secret como string ou value para JsonSchema)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterWebhookParams {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    #[schemars(
        description = "e.g. subscriptionAccount.created, userAccount.updated"
    )]
    pub trigger: String,
    #[schemars(description = "Optional HTTP method: POST, PUT, PATCH, DELETE")]
    pub method: Option<String>,
    #[schemars(description = "Optional secret (JSON object)")]
    pub secret: Option<serde_json::Value>,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWebhooksParams {
    pub name: Option<String>,
    #[schemars(description = "e.g. subscriptionAccount.created")]
    pub trigger: Option<String>,
    pub page_size: Option<i32>,
    pub skip: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookParams {
    pub webhook_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    #[schemars(description = "Optional secret (JSON object)")]
    pub secret: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWebhookParams {
    pub webhook_id: Uuid,
}
