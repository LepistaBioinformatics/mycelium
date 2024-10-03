use crate::domain::{
    actors::ActorName, dtos::profile::Profile, entities::RoleDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Delete a single role.
#[tracing::instrument(name = "delete_role", skip(profile, role_deletion_repo))]
pub async fn delete_role(
    profile: Profile,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn RoleDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile.get_default_delete_ids_or_error(vec![
        ActorName::GuestManager.to_string(),
    ])?;

    // ? ----------------------------------------------------------------------
    // ? Persist Role
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}
