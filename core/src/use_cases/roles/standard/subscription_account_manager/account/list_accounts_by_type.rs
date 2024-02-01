use crate::{
    domain::{
        actors::DefaultActor,
        dtos::{
            account::{Account, AccountTypeEnum},
            profile::Profile,
        },
        entities::{AccountFetching, AccountTypeRegistration},
        utils::try_as_uuid,
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use mycelium_base::{
    entities::{FetchManyResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
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
    tag_value: Option<String>,
    page_size: Option<i32>,
    skip: Option<i32>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_type_registration: Box<&dyn AccountTypeRegistration>,
) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_view_ids_or_error(vec![
        DefaultActor::SubscriptionAccountManager.to_string(),
    ])?;

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

    let (updated_term, account_id) = {
        if let Some(i) = term {
            match try_as_uuid(&i) {
                Ok(id) => (None, Some(id)),
                Err(_) => (Some(i), None),
            }
        } else {
            (None, None)
        }
    };

    let (updated_tag, tag_id) = {
        if let Some(i) = tag_value {
            match try_as_uuid(&i) {
                Ok(id) => (None, Some(id)),
                Err(_) => (Some(i), None),
            }
        } else {
            (None, None)
        }
    };

    account_fetching_repo
        .list(
            updated_term,
            is_owner_active,
            is_account_active,
            is_account_checked,
            is_account_archived,
            tag_id,
            updated_tag,
            account_id,
            account_type_id,
            Some(is_subscription.unwrap_or(false)),
            page_size,
            skip,
        )
        .await
}
