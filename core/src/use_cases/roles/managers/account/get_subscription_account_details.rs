use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::AccountFetching,
};

use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Get details of a single subscription account
///
/// These details could include information about guest accounts, modifications
/// and others.
pub async fn get_subscription_account_details(
    profile: Profile,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
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
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    match account_fetching_repo.get(account_id).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(id) => return Ok(FetchResponseKind::NotFound(id)),
            FetchResponseKind::Found(account) => match account.to_owned().account_type {
                ParentEnum::Id(_) => return Err(use_case_err(
                    "Could not check account type validity. Please contact administrators."
                        .to_string(),
                    Some(true),
                    None,
                )),
                ParentEnum::Record(account_type) => {
                    if !account_type.is_subscription {
                        return Err(use_case_err(
                            "Provided account ID is not from a subscription."
                                .to_string(),
                            Some(true),
                            None,
                        ))
                    }

                    Ok(FetchResponseKind::Found(account))
                }
            }
        }
    }
}
