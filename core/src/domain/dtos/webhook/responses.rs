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
pub struct WebHookPropagationResponse<PayloadBody: Serialize + Clone> {
    /// The propagated payload
    ///
    /// This is the payload that is sent to the webhook. It should be a
    /// serializable object. The key is flattened to the root of the object,
    /// then the value is serialized as the value of the key.
    ///
    #[serde(flatten)]
    pub payload: PayloadBody,

    /// Propagation responses from the webhooks
    ///
    /// This is the response from the webhooks. It contains the url, status
    /// code, and the body of the response. If the body is not present, it
    /// should be `None` and should be skipped on serialization.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagations: Option<Vec<HookResponse>>,
}
