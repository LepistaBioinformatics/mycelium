use crate::domain::{
    dtos::{account::AccountDTO, profile::ProfileDTO},
    entities::{
        account_fetching::AccountFetching, account_updating::AccountUpdating,
    },
};

use clean_base::{
    entities::default_response::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

/// Update the own account.
///
/// This function uses the id of the Profile to fetch and update the account
/// name, allowing only the account owner to update the account name.
pub async fn update_own_account_name(
    profile: ProfileDTO,
    name: String,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<AccountDTO>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the account
    // ? -----------------------------------------------------------------------

    let mut account =
        match account_fetching_repo.get(profile.current_account_id).await {
            Err(err) => return Err(err),
            Ok(res) => match res {
                FetchResponseKind::NotFound(id) => {
                    return Err(use_case_err(
                        format!("Invalid account id: {}", id.unwrap()),
                        Some(true),
                        None,
                    ))
                }
                FetchResponseKind::Found(res) => res,
            },
        };

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account.name = name;

    account_updating_repo.update(account).await
}
