use crate::domain::{
    dtos::{guest::UserRoleDTO, profile::ProfileDTO},
    entities::{
        manager::user_role_deletion::UserRoleDeletion,
        shared::default_responses::DeleteResponse,
    },
    utils::errors::MappedErrors,
};

use uuid::Uuid;

/// This function deletes a single role. Only manager user could execute such
/// operation.
pub async fn delete_role(
    profile: ProfileDTO,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn UserRoleDeletion>,
) -> Result<DeleteResponse<UserRoleDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check user permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(MappedErrors::new(
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
