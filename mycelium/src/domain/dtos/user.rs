use super::email::EmailDTO;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDTO {
    pub id: Option<Uuid>,

    pub username: String,
    pub email: EmailDTO,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}
