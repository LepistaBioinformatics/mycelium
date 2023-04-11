use super::try_to_reach_desired_status::try_to_reach_desired_status;
use crate::domain::{
    dtos::{
        account::{Account, VerboseStatus},
        profile::Profile,
    },
    entities::{AccountFetching, AccountUpdating},
};

use clean_base::{
    dtos::enums::ParentEnum::*,
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Approve new created accounts.
///
/// This action is needed when a new account is created but not approved by a
/// system administrator. Only checked accounts could perform actions over the
/// system.
pub async fn change_account_approval_status(
    profile: Profile,
    account_id: Uuid,
    is_approved: bool,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch target account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo.get(account_id).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Invalid account ID: {:?}", id))
                .as_error()
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    // Check if the account id os Some. Case false the operation is prohibited.
    let target_account_id = match account.id {
        None => {
            return use_case_err(format!(
                "Prohibited operation. Target account ({account_id}) could 
not be checked."
            ))
            .as_error()
        }
        Some(res) => res,
    };

    // Check if the account that will perform approve action has enough
    // privileges.
    if ![profile.is_manager, profile.is_staff]
        .into_iter()
        .any(|i| i == true)
    {
        return use_case_err(format!(
            "Not enough permissions approve account `{target_account_id}`."
        ))
        .as_error();
    }

    // Check if the target account to be changed is a Standard account.
    match account.to_owned().account_type {
        Id(_) => {
            return use_case_err(format!(
                "Prohibited operation. Account type of the target account 
({account_id}) could not be checked."
            ))
            .as_error()
        }
        Record(res) => {
            if profile.is_manager && !profile.is_staff && res.is_staff {
                return use_case_err(String::from(
                    "Prohibited operation. Managers could not perform 
editions on accounts with more privileges than himself.",
                ))
                .as_error();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update account status
    // ? -----------------------------------------------------------------------

    let updated_account = try_to_reach_desired_status(
        account.to_owned(),
        match is_approved {
            true => VerboseStatus::Active,
            false => VerboseStatus::Archived,
        },
    )
    .await?;

    account_updating_repo.update(updated_account).await
}
