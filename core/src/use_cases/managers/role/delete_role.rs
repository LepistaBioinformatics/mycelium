use crate::domain::{
    dtos::profile::ProfileDTO, entities::role_deletion::RoleDeletion,
};

use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Delete a single role.
pub async fn delete_role(
    profile: ProfileDTO,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn RoleDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "The current user has no sufficient privileges to delete roles."
                .to_string(),
            Some(true),
            None,
        ));
    }

    // ? ----------------------------------------------------------------------
    // ? Persist Role
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}
