use crate::domain::{
    dtos::{email::Email, guest_role::Permission, profile::Profile},
    entities::GuestUserDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Uninvite user to perform a role actions from account
///
#[tracing::instrument(
    name = "revoke_user_guest_to_subscription_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
        guest_email = %email.redacted_email(),
    ),
    skip(
        profile,
        email,
        guest_user_deletion_repo,
    )
)]
pub async fn revoke_user_guest_to_subscription_manager_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    guest_role_id: Uuid,
    email: Email,
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
        .get_tenant_wide_permission_or_error(tenant_id, Permission::Write)?;

    // ? -----------------------------------------------------------------------
    // ? Uninvite guest
    // ? -----------------------------------------------------------------------

    guest_user_deletion_repo
        .delete(guest_role_id, account_id, email.to_string())
        .await
}
