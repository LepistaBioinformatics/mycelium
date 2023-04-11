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
    entities::{FetchManyResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// List account given an account-type
///
/// Get a list of available accounts given the AccountTypeEnum.
pub async fn list_accounts_by_type(
    profile: Profile,
    term: Option<String>,
    is_owner_active: Option<bool>,
    is_account_active: Option<bool>,
    is_account_checked: Option<bool>,
    is_account_archived: Option<bool>,
    is_subscription: Option<bool>,
    page_size: Option<i32>,
    skip: Option<i32>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_type_registration: Box<&dyn AccountTypeRegistration>,
) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to list 
            subscription accounts."
                .to_string(),
        )
        .as_error();
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
    .await?
    {
        GetOrCreateResponseKind::NotCreated(res, _) => res.id,
        GetOrCreateResponseKind::Created(res) => res.id,
    };

    // ? -----------------------------------------------------------------------
    // ? List accounts
    // ? -----------------------------------------------------------------------

    account_fetching_repo
        .list(
            term,
            is_owner_active,
            is_account_active,
            is_account_checked,
            is_account_archived,
            account_type_id,
            Some(is_subscription.unwrap_or(false)),
            page_size,
            skip,
        )
        .await
}
