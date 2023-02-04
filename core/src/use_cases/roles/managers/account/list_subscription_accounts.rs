use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            profile::Profile,
        },
        entities::{AccountFetching, AccountTypeRegistration},
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use clean_base::{
    entities::default_response::{
        FetchManyResponseKind, GetOrCreateResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};

/// List subscription accounts
///
/// Get a list of available subscription accounts.
pub async fn list_subscription_accounts(
    profile: Profile,
    name: Option<String>,
    is_active: Option<bool>,
    is_checked: Option<bool>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_type_registration: Box<&dyn AccountTypeRegistration>,
) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "The current user has no sufficient privileges to register 
            subscription accounts."
                .to_string(),
            Some(true),
            None,
        ));
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch account-type id
    // ? -----------------------------------------------------------------------

    let account_type_id = match get_or_create_default_account_types(
        AccountTypeEnum::Subscription,
        None,
        None,
        account_type_registration,
    )
    .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(res, _) => res.id,
            GetOrCreateResponseKind::Created(res) => res.id,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? List accounts
    // ? -----------------------------------------------------------------------

    account_fetching_repo
        .list(name, is_active, is_checked, account_type_id)
        .await
}
