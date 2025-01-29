use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
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
pub struct WebHookPropagationArtifact {
    /// The propagated payload
    ///
    /// This is the payload that is sent to the webhook. It should be a
    /// serializable object. The key is flattened to the root of the object,
    /// then the value is serialized as the value of the key.
    ///
    pub payload: String,

    /// Propagation responses from the webhooks
    ///
    /// This is the response from the webhooks. It contains the url, status
    /// code, and the body of the response. If the body is not present, it
    /// should be `None` and should be skipped on serialization.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagations: Option<Vec<HookResponse>>,
}

impl WebHookPropagationArtifact {
    /// Enccode payload as base64
    ///
    /// Stringify with serde and envode the payload as base64.
    ///
    pub fn encode_payload(&self) -> Result<Self, MappedErrors> {
        let serialized_payload =
            serde_json::to_string(&self.payload).map_err(|e| {
                dto_err(format!("Failed to serialize payload: {}", e))
            })?;

        let encoded_payload =
            general_purpose::STANDARD.encode(serialized_payload.as_bytes());

        Ok(WebHookPropagationArtifact {
            payload: encoded_payload,
            propagations: self.propagations.clone(),
        })
    }

    /// Decode payload from base64
    ///
    /// Decode the payload from base64 and return the original payload.
    ///
    pub fn decode_payload(
        payload: WebHookPropagationArtifact,
    ) -> Result<WebHookPropagationArtifact, MappedErrors> {
        let decoded_payload =
            match general_purpose::STANDARD.decode(&payload.payload) {
                Err(_) => return dto_err("Failed to decode base64").as_error(),
                Ok(decoded) => String::from_utf8(decoded)
                    .map_err(|_| dto_err("Failed to decode payload"))?,
            };

        let payload = serde_json::from_str(&decoded_payload).map_err(|e| {
            dto_err(format!("Failed to deserialize payload: {}", e))
        })?;

        Ok(WebHookPropagationArtifact {
            payload,
            propagations: None,
        })
    }
}
