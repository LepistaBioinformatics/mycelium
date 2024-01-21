use super::try_to_reach_desired_status::try_to_reach_desired_status;
use crate::domain::{
    actors::DefaultActor,
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

/// Change archival status of and account.
///
/// After created new accounts could be approved or archived. Case archived
/// these use-case should be used.
pub async fn change_account_archival_status(
    profile: Profile,
    account_id: Uuid,
    is_archived: bool,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::UserAccountManager.to_string(),
    ])?;

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
    // ? Prevent self privilege escalation
    // ? -----------------------------------------------------------------------

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
        match is_archived {
            true => VerboseStatus::Archived,
            false => VerboseStatus::Pending,
        },
    )
    .await?;

    account_updating_repo.update(updated_account).await
}
