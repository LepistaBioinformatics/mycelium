use super::issuable_notification_service::IssuableNotificationService;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// What the caller can issue notification connection strings for
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationIssuanceOptions {
    /// The instance cap on the string expiration, in days
    pub max_connection_string_ttl_in_days: u32,

    /// The services the caller can issue for, sorted by display name
    pub services: Vec<IssuableNotificationService>,
}
