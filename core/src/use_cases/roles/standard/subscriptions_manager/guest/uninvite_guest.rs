use crate::domain::{
    actors::ActorName,
    dtos::{
        native_error_codes::NativeErrorCodes, profile::Profile,
        related_accounts::RelatedAccounts,
    },
    entities::GuestUserDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Uninvite user to perform a role actions from account
///
#[tracing::instrument(
    name = "uninvite_guest",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn uninvite_guest(
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

    if let RelatedAccounts::AllowedAccounts(allowed_ids) = &profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionsManager.to_string(),
        ])?
    {
        if !allowed_ids.contains(&account_id) {
            return use_case_err(
                "User is not allowed to perform this action".to_string(),
            )
            .with_code(NativeErrorCodes::MYC00013)
            .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Uninvite guest
    // ? -----------------------------------------------------------------------

    guest_user_deletion_repo
        .delete(guest_role_id, account_id, email)
        .await
}
