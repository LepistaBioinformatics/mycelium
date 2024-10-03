use crate::domain::{
    actors::ActorName,
    dtos::{
        guest::GuestUser, native_error_codes::NativeErrorCodes,
        profile::Profile, related_accounts::RelatedAccounts,
    },
    entities::GuestUserOnAccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
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
    tenant_id: Uuid,
    account_id: Uuid,
    old_guest_user_id: Uuid,
    new_guest_user_id: Uuid,
    guest_user_on_account_updating_repo: Box<&dyn GuestUserOnAccountUpdating>,
) -> Result<UpdatingResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if let RelatedAccounts::AllowedAccounts(allowed_ids) = &profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_update_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
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
    // ? Update role
    // ? -----------------------------------------------------------------------

    guest_user_on_account_updating_repo
        .update(account_id, old_guest_user_id, new_guest_user_id)
        .await
}
