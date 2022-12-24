use super::guest::PermissionsType;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResourcesDTO {
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    pub guest_account_id: Uuid,
    pub role: String,
    pub permissions: Vec<PermissionsType>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

/// This object should be used over the application layer operations.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDTO {
    pub email: String,
    pub current_account_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,

    /// If the licensed IDs are `None`, the user has only permissions to act
    /// inside their own account.
    pub licensed_resources: Option<Vec<LicensedResourcesDTO>>,
}
