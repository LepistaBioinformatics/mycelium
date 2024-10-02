use crate::domain::{
    actors::DefaultActor,
    dtos::{guest::GuestUser, profile::Profile},
    entities::GuestUserOnAccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Update the user's guest role.
///
/// This use case is used to replace the user's guest role. The user's guest
/// role is the role that the user has in the account.
///
#[tracing::instrument(
    name = "update_user_guest_role",
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn update_user_guest_role(
    profile: Profile,
    account_id: Uuid,
    old_guest_user_id: Uuid,
    new_guest_user_id: Uuid,
    guest_user_on_account_updating_repo: Box<&dyn GuestUserOnAccountUpdating>,
) -> Result<UpdatingResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::TenantOwner.to_string(),
        DefaultActor::TenantManager.to_string(),
        DefaultActor::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Update role
    // ? -----------------------------------------------------------------------

    guest_user_on_account_updating_repo
        .update(account_id, old_guest_user_id, new_guest_user_id)
        .await
}
