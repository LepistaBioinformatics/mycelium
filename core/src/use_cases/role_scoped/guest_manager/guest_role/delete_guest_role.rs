use crate::domain::{
    actors::SystemActor, dtos::profile::Profile, entities::GuestRoleDeletion,
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
    guest_role_id: Uuid,
    role_deletion_repo: Box<&dyn GuestRoleDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check user permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? ----------------------------------------------------------------------
    // ? Perform the deletion operation
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(guest_role_id).await
}
