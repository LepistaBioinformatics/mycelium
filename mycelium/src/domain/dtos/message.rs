use super::email::EmailDTO;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageDTO {
    // Addresses
    pub from: EmailDTO,
    pub to: EmailDTO,
    pub cc: Option<EmailDTO>,

    // Message
    pub subject: String,
    pub message_head: Option<String>,
    pub message_body: String,
    pub message_footer: Option<String>,
}
