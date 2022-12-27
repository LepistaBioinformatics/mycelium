use crate::domain::{
    dtos::{guest::GuestRoleDTO, profile::ProfileDTO},
    entities::guest_role_deletion::GuestRoleDeletion,
};

use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function deletes a single role. Only manager user could execute such
/// operation.
pub async fn delete_guest_role(
    profile: ProfileDTO,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn GuestRoleDeletion>,
) -> Result<DeletionResponseKind<GuestRoleDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check user permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "Only manager users could perform such operation.".to_string(),
            Some(true),
            None,
        ));
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the deletion operation
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}
