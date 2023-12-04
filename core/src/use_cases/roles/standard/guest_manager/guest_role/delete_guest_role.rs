use crate::domain::{dtos::profile::Profile, entities::GuestRoleDeletion};

use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function deletes a single role. Only manager user could execute such
/// operation.
pub async fn delete_guest_role(
    profile: Profile,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn GuestRoleDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check user permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "Only manager users could perform such operation.".to_string(),
        )
        .as_error();
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the deletion operation
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}
