use super::email::Email;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
