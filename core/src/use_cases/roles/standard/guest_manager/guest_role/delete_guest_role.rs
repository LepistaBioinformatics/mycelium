use crate::domain::{
    actors::ActorName, dtos::profile::Profile, entities::GuestRoleDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// This function deletes a single role. Only manager user could execute such
/// operation.
#[tracing::instrument(name = "delete_guest_role", skip_all)]
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

    profile.get_default_delete_ids_or_error(vec![
        ActorName::GuestManager.to_string()
    ])?;

    // ? ----------------------------------------------------------------------
    // ? Perform the deletion operation
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}
