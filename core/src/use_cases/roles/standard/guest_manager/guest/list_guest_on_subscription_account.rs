use clean_base::{
    dtos::enums::ParentEnum,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

use crate::domain::{
    actors::DefaultActor,
    dtos::{guest::GuestUser, profile::Profile},
    entities::{AccountFetching, GuestUserFetching},
};

/// List guests on subscription account
///
/// Fetch a list of the guest accounts associated with a single subscription
/// account.
pub async fn list_guest_on_subscription_account(
    profile: Profile,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_fetching_repo: Box<&dyn GuestUserFetching>,
) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .get_view_ids_or_error(vec![DefaultActor::GuestManager.to_string()])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch the target subscription account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo.get(account_id).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Invalid account ID: {}", id.unwrap()))
                .as_error()
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the account is a valid subscription account
    // ? -----------------------------------------------------------------------

    match account.account_type {
        ParentEnum::Id(id) => {
            return use_case_err(format!("Invalid account ID: {}", id))
                .as_error()
        }
        ParentEnum::Record(account_type) => {
            if !account_type.is_subscription {
                return use_case_err(format!("Account is not subscription."))
                    .as_error();
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch guest users
    // ? -----------------------------------------------------------------------

    guest_user_fetching_repo.list(account_id).await
}
