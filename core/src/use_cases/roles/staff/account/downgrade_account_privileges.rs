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

use clean_base::{
    dtos::enums::ParentEnum,
    entities::{
        FetchResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
    },
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Downgrade the account status.
///
/// This action should be used to downgrade Standard and Manager accounts.
/// Subscription and Staff accounts should not be downgraded.
pub async fn downgrade_account_privileges(
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
            "The current user has no sufficient privileges to downgrade 
            accounts."
                .to_string(),
            Some(true),
            None,
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Check if the account type if allowed
    // ? -----------------------------------------------------------------------

    if !vec![AccountTypeEnum::Standard, AccountTypeEnum::Manager]
        .contains(&target_account_type)
    {
        return use_case_err(
            String::from("Invalid upgrade target."),
            Some(true),
            None,
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch the account
    // ? -----------------------------------------------------------------------

    let mut account = match account_fetching_repo.get(account_id).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(id) => {
                return use_case_err(
                    format!("Invalid account id: {}", id.unwrap()),
                    Some(true),
                    None,
                )
            }
            FetchResponseKind::Found(res) => res,
        },
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
    .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(account_type, _) => {
                account_type
            }
            GetOrCreateResponseKind::Created(account_type) => account_type,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account.account_type = ParentEnum::Record(account_type);

    account_updating_repo.update(account).await
}
