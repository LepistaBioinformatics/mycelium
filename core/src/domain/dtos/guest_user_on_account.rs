use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserOnAccount {
    /// The guest user id
    pub guest_user_id: Uuid,

    /// The account id
    pub account_id: Uuid,

    /// The created date
    pub created: DateTime<Local>,

    /// The last updated date
    pub permit_flags: Vec<String>,

    /// The deny flags
    pub deny_flags: Vec<String>,
}
