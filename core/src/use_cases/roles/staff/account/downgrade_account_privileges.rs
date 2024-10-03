use crate::domain::{
    dtos::{account::Account, account_type::AccountTypeV2, profile::Profile},
    entities::AccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Downgrade the account status.
///
/// This action should be used to downgrade Standard and Manager accounts.
/// Subscription and Staff accounts should not be downgraded.
#[tracing::instrument(name = "downgrade_account_privileges", skip_all)]
pub async fn downgrade_account_privileges(
    profile: Profile,
    account_id: Uuid,
    target_account_type: AccountTypeV2,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    //
    // Only staff users should perform such action.
    // ? -----------------------------------------------------------------------

    if !profile.is_staff {
        return use_case_err(
            "The current user has no sufficient privileges to downgrade 
            accounts.",
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Check if the account type if allowed
    // ? -----------------------------------------------------------------------

    if !vec![AccountTypeV2::User, AccountTypeV2::Manager]
        .contains(&target_account_type)
    {
        return use_case_err("Invalid upgrade target.").as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account_updating_repo
        .update_account_type(account_id, target_account_type)
        .await
}
