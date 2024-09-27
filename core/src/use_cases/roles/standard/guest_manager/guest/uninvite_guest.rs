use crate::domain::{
    actors::DefaultActor, dtos::profile::Profile, entities::GuestUserDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Uninvite user to perform a role actions from account
///
#[tracing::instrument(name = "uninvite_guest", skip_all)]
pub async fn uninvite_guest(
    profile: Profile,
    account_id: Uuid,
    guest_role_id: Uuid,
    email: String,
    guest_user_deletion_repo: Box<&dyn GuestUserDeletion>,
) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::GuestManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Uninvite guest
    // ? -----------------------------------------------------------------------

    guest_user_deletion_repo
        .delete(guest_role_id, account_id, email)
        .await
}
