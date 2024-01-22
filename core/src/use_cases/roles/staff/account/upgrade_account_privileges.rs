use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            profile::Profile,
        },
        entities::{AccountFetching, AccountTypeRegistration, AccountUpdating},
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use mycelium_base::{
    dtos::Parent,
    entities::{
        FetchResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Upgrade the account status.
///
/// This action should be used to upgrade Standard, Manager, and Staff accounts.
/// Subscription accounts should not be upgraded.
pub async fn upgrade_account_privileges(
    profile: Profile,
    account_id: Uuid,
    target_account_type: AccountTypeEnum,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
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

    if !vec![AccountTypeEnum::Manager, AccountTypeEnum::Staff]
        .contains(&target_account_type)
    {
        return use_case_err(String::from("Invalid upgrade target."))
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch the account
    // ? -----------------------------------------------------------------------

    let mut account = match account_fetching_repo.get(account_id).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Invalid account id: {}", id.unwrap()))
                .as_error()
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch account type
    //
    // Get or create the default account-type.
    // ? -----------------------------------------------------------------------

    let account_type = match get_or_create_default_account_types(
        target_account_type,
        None,
        None,
        account_type_registration_repo,
    )
    .await?
    {
        GetOrCreateResponseKind::NotCreated(account_type, _) => account_type,
        GetOrCreateResponseKind::Created(account_type) => account_type,
    };

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account.account_type = Parent::Record(account_type);

    account_updating_repo.update(account).await
}
