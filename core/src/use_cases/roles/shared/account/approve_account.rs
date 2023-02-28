use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::{AccountFetching, AccountUpdating},
};

use clean_base::{
    dtos::enums::ParentEnum::*,
    entities::default_response::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Check the account as approved.
///
/// This action is needed when a new account is created but not approved by a
/// system administrator. Only checked accounts could perform actions over the
/// system.
pub async fn approve_account(
    profile: Profile,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch target account
    // ? -----------------------------------------------------------------------

    let mut account = match account_fetching_repo.get(account_id).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(id) => {
                return Err(use_case_err(
                    format!("Invalid account ID: {:?}", id),
                    Some(true),
                    None,
                ))
            }
            FetchResponseKind::Found(res) => res,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    // Check if the account id os Some. Case false the operation is prohibited.
    let target_account_id = match account.id {
        None => {
            return Err(use_case_err(
                format!(
                    "Prohibited operation. Target account ({account_id}) could 
                    not be checked."
                ),
                Some(true),
                None,
            ))
        }
        Some(res) => res,
    };

    // Check if the account that will perform approve action has enough
    // privileges.
    if ![profile.is_manager, profile.is_staff]
        .into_iter()
        .any(|i| i == true)
    {
        return Err(use_case_err(
            format!(
                "Not enough permissions approve the account 
                {target_account_id}."
            ),
            Some(true),
            None,
        ));
    }

    // Check if the target account to be changed is a Standard account.
    match account.to_owned().account_type {
        Id(_) => {
            return Err(use_case_err(
                format!(
                    "Prohibited operation. Account type of the target account 
                    ({account_id}) could not be checked."
                ),
                Some(true),
                None,
            ))
        }
        Record(res) => {
            if profile.is_manager && res.is_staff {
                return Err(use_case_err(
                    String::from(
                        "Prohibited operation. Managers could not perform 
                        editions on accounts with more privileges than himself.",
                    ),
                    Some(true),
                    None,
                ));
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update account status
    // ? -----------------------------------------------------------------------

    account.is_checked = true;

    account_updating_repo.update(account).await
}
