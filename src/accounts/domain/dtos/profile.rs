use super::{email::EmailDTO, guest::PermissionsType};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicensedIdentifiersDTO {
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    pub account_id: Uuid,
    pub application_id: Uuid,
    pub permissions: Vec<PermissionsType>,
}

/// The profile-dto is an special case of object. Different from other DTOs, it
/// has not a database representation. Instead, this object is generated only
/// during operations of licensing emails to perform operations to specific
/// records, based on the record owner ID.
///
/// This object should be used over the application layer operations.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDTO {
    pub email: EmailDTO,
    pub account: Uuid,

    pub name: String,
    pub description: String,
    pub is_manager: bool,
    pub is_staff: bool,

    /// If the licensed IDs are `None`, the user has only permissions to act
    /// inside their own account.
    pub licensed_ids: Option<Vec<LicensedIdentifiersDTO>>,
}
