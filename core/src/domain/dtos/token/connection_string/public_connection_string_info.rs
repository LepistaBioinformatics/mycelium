use crate::domain::dtos::{email::Email, token::ConnectionStringBean};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublicConnectionStringInfo {
    /// The unique identifier of the token
    pub id: u32,

    /// The inner identifier of the token
    pub inner_id: Uuid,

    /// The account id of the token
    pub account_id: Uuid,

    /// The email of the token
    pub email: Email,

    /// The name of the token
    pub name: String,

    /// The expiration date time
    pub expiration: DateTime<Local>,

    /// The creation date time of the token
    pub created_at: DateTime<Local>,

    /// The scope of the token
    pub scope: Vec<ConnectionStringBean>,
}
