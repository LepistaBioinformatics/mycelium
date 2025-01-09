use crate::domain::{
    dtos::{account::Account, account_type::AccountType, profile::Profile},
    entities::AccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Upgrade the account status.
///
/// This action should be used to upgrade Standard, Manager, and Staff accounts.
/// Subscription accounts should not be upgraded.
#[tracing::instrument(name = "upgrade_account_privileges", skip_all)]
pub async fn upgrade_account_privileges(
    profile: Profile,
    account_id: Uuid,
    target_account_type: AccountType,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    //
    // Only staff users should perform such action.
    // ? -----------------------------------------------------------------------

    if !profile.is_staff {
        return use_case_err(
            "The current user has no sufficient privileges to upgrade 
            accounts."
                .to_string(),
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Check if the account type if allowed
    // ? -----------------------------------------------------------------------

    if !vec![AccountType::Manager, AccountType::Staff]
        .contains(&target_account_type)
    {
        return use_case_err(String::from("Invalid upgrade target."))
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account_updating_repo
        .update_account_type(account_id, target_account_type)
        .await
}
