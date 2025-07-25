use crate::domain::{
    actors::SystemActor, dtos::profile::Profile, entities::GuestUserDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Uninvite user to perform a role actions from account
///
#[tracing::instrument(
    name = "revoke_user_guest_to_subscription_account",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn revoke_user_guest_to_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
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

    profile
        .on_tenant(tenant_id)
        .on_account(account_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_accounts_or_tenant_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Uninvite guest
    // ? -----------------------------------------------------------------------

    guest_user_deletion_repo
        .delete(guest_role_id, account_id, email)
        .await
}
