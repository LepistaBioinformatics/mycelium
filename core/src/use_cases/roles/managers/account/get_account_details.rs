use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::AccountFetching,
};

use clean_base::{
    entities::FetchResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Get details of a single account
///
/// These details could include information about guest accounts, modifications
/// and others.
pub async fn get_account_details(
    profile: Profile,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register 
            subscription accounts."
                .to_string(),
            Some(true),
            None,
        );
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    account_fetching_repo.get(account_id).await
}
