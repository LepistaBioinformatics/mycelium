use crate::domain::{
    actors::DefaultActor, dtos::profile::Profile, entities::GuestUserDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Uninvite user to perform a role actions from account
///
#[tracing::instrument(
    name = "uninvite_guest",
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn uninvite_guest(
    profile: Profile,
    account_id: Uuid,
    guest_role_id: Uuid,
    email: String,
    guest_user_deletion_repo: Box<&dyn GuestUserDeletion>,
) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    //
    // Despite the action itself is a deletion one, user must have the
    // permission to update the guest account.
    //
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::TenantOwner.to_string(),
        DefaultActor::TenantManager.to_string(),
        DefaultActor::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Uninvite guest
    // ? -----------------------------------------------------------------------

    guest_user_deletion_repo
        .delete(guest_role_id, account_id, email)
        .await
}
