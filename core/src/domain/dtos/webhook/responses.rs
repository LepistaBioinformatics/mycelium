use super::WebHookTrigger;

use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WebHookExecutionStatus {
    Pending,
    Success,
    Failed,
    Unknown,
}

impl Display for WebHookExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl FromStr for WebHookExecutionStatus {
    type Err = MappedErrors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "unknown" => Ok(Self::Unknown),
            _ => dto_err("Invalid webhook execution status").as_error(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    pub url: String,
    pub status: u16,
    pub body: Option<String>,
    pub datetime: DateTime<Local>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebHookPayloadArtifact {
    /// The id of the webhook payload artifact
    ///
    /// This is the id of the webhook payload artifact. It is the id that is
    /// used to identify the webhook payload artifact.
    ///
    pub id: Option<Uuid>,

    /// The propagated payload
    ///
    /// This is the payload that is sent to the webhook. It should be a
    /// serializable object. The key is flattened to the root of the object,
    /// then the value is serialized as the value of the key.
    ///
    pub payload: String,

    /// The trigger of the webhook
    ///
    /// This is the trigger of the webhook. It is the trigger that is used to
    /// determine if the webhook should be executed.
    ///
    pub trigger: WebHookTrigger,

    /// Propagation responses from the webhooks
    ///
    /// This is the response from the webhooks. It contains the url, status
    /// code, and the body of the response. If the body is not present, it
    /// should be `None` and should be skipped on serialization.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagations: Option<Vec<HookResponse>>,

    /// Encrypted payload
    ///
    /// If the payload is encrypted, this should be set to true. Otherwise,
    /// it should be set to false or None.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted: Option<bool>,

    /// The number of attempts to dispatch the webhook
    ///
    /// This is the number of attempts to dispatch the webhook. It is the number
    /// of attempts that have been made to dispatch the webhook.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<u8>,

    /// The attempted at timestamp
    ///
    /// This is the timestamp when the webhook payload artifact was attempted.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempted: Option<DateTime<Local>>,

    /// The created at timestamp
    ///
    /// This is the timestamp when the webhook payload artifact was created.
    ///
    pub created: Option<DateTime<Local>>,

    /// The status of the webhook execution
    ///
    /// This is the status of the webhook execution. It is the status that is
    /// used to determine if the webhook should be executed.
    ///
    pub status: Option<WebHookExecutionStatus>,
}

impl WebHookPayloadArtifact {
    pub fn new(
        id: Option<Uuid>,
        payload: String,
        trigger: WebHookTrigger,
    ) -> Self {
        Self {
            id,
            payload,
            trigger,
            propagations: None,
            encrypted: None,
            attempts: None,
            attempted: None,
            created: None,
            status: Some(WebHookExecutionStatus::Pending),
        }
    }

    /// Encode payload as base64
    ///
    /// Stringify with serde and encode the payload as base64.
    ///
    pub fn encode_payload(&mut self) -> Result<Self, MappedErrors> {
        let serialized_payload =
            serde_json::to_string(&self.payload).map_err(|e| {
                dto_err(format!("Failed to serialize payload: {}", e))
            })?;

        let encoded_payload =
            general_purpose::STANDARD.encode(serialized_payload.as_bytes());

        Ok(Self {
            payload: encoded_payload,
            ..self.clone()
        })
    }

    /// Decode payload from base64
    ///
    /// Decode the payload from base64 and return the original payload.
    ///
    pub fn decode_payload(
        &self,
    ) -> Result<WebHookPayloadArtifact, MappedErrors> {
        let decoded_payload =
            match general_purpose::STANDARD.decode(&self.payload) {
                Err(_) => return dto_err("Failed to decode base64").as_error(),
                Ok(decoded) => String::from_utf8(decoded)
                    .map_err(|_| dto_err("Failed to decode payload"))?,
            };

        let payload = serde_json::from_str(&decoded_payload).map_err(|e| {
            dto_err(format!("Failed to deserialize payload: {}", e))
        })?;

        Ok(Self {
            payload,
            ..self.clone()
        })
    }
}
