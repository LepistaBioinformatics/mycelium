use crate::domain::{
    dtos::{guest::GuestRoleDTO, profile::ProfileDTO},
    entities::{
        manager::user_role_deletion::UserRoleDeletion,
        shared::default_responses::DeleteResponse,
    },
    utils::errors::{use_case_err, MappedErrors},
};

use uuid::Uuid;

/// This function deletes a single role. Only manager user could execute such
/// operation.
pub async fn delete_guest_role(
    profile: ProfileDTO,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn UserRoleDeletion>,
) -> Result<DeleteResponse<GuestRoleDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check user permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "Only manager user could perform such operation.".to_string(),
            Some(true),
            None,
        ));
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the deletion operation
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}
