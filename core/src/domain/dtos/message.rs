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
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum MessageStatus {
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

impl Default for MessageStatus {
    fn default() -> Self {
        MessageStatus::Queued
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
