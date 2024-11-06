use crate::domain::dtos::account::Account;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    pub url: String,
    pub status: u16,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountPropagationWebHookResponse {
    /// The account that was propagated.
    pub account: Account,

    /// Responses from the webhooks.
    pub propagation_responses: Option<Vec<HookResponse>>,
}
