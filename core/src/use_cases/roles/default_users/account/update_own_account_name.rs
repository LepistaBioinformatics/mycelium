use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::{AccountFetching, AccountUpdating},
};

use clean_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// Update the own account.
///
/// This function uses the id of the Profile to fetch and update the account
/// name, allowing only the account owner to update the account name.
pub async fn update_own_account_name(
    profile: Profile,
    name: String,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the account
    // ? -----------------------------------------------------------------------

    let mut account = match account_fetching_repo
        .get(profile.current_account_id)
        .await?
    {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(
                format!("Invalid account id: {}", id.unwrap()),
                Some(true),
                None,
            )
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account.name = name;

    account_updating_repo.update(account).await
}
