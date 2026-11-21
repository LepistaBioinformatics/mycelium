use super::email::Email;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum FromEmail {
    Email(Email),
    NamedEmail(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    // Addresses
    pub from: FromEmail,
    pub to: Email,
    pub cc: Option<Email>,

    // Message
    pub subject: String,
    pub body: String,

    /// Extra mail headers, as `(name, value)` pairs. Only `X-Mycelium-*`
    /// names are delivered (see `check_message_header`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum MessageStatus {
    #[default]
    Queued,
    Sent,
    Failed,
}

impl Display for MessageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for MessageStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(MessageStatus::Queued),
            "sent" => Ok(MessageStatus::Sent),
            "failed" => Ok(MessageStatus::Failed),
            _ => Err(format!("Invalid message status: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageSendingEvent {
    pub id: Uuid,
    pub message: Message,
    pub created: DateTime<Local>,
    pub attempted: Option<DateTime<Local>>,
    pub status: MessageStatus,
    pub attempts: i32,
    pub error: Option<String>,
}

impl MessageSendingEvent {
    pub fn new(message: Message) -> Self {
        Self {
            id: Uuid::new_v4(),
            message,
            created: Local::now(),
            attempted: None,
            status: MessageStatus::Queued,
            attempts: 0,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRE_HEADERS_MESSAGE: &str = r#"{
        "from": {"namedEmail": "Mycelium <noreply@example.com>"},
        "to": {"username": "user", "domain": "example.com"},
        "cc": null,
        "subject": "Welcome",
        "body": "<p>Hello</p>"
    }"#;

    #[test]
    fn message_stored_before_headers_existed_still_deserializes() {
        let message: Message =
            serde_json::from_str(PRE_HEADERS_MESSAGE).unwrap();

        assert!(message.headers.is_empty());
    }

    #[test]
    fn empty_headers_are_not_serialized() {
        let message: Message =
            serde_json::from_str(PRE_HEADERS_MESSAGE).unwrap();

        let json = serde_json::to_value(&message).unwrap();

        assert!(json.get("headers").is_none());
    }

    #[test]
    fn headers_round_trip() {
        let mut message: Message =
            serde_json::from_str(PRE_HEADERS_MESSAGE).unwrap();
        message.headers = vec![(
            "X-Mycelium-Source-Service".to_string(),
            "service-01".to_string(),
        )];

        let json = serde_json::to_string(&message).unwrap();
        let decoded: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.headers, message.headers);
    }
}
