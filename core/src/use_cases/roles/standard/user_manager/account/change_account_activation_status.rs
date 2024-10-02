use super::try_to_reach_desired_status::try_to_reach_desired_status;
use crate::domain::{
    actors::DefaultActor,
    dtos::{
        account::{Account, VerboseStatus},
        profile::Profile,
    },
    entities::{AccountFetching, AccountUpdating},
};

use mycelium_base::{
    dtos::Parent::*,
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Change activation status of the target account.
#[tracing::instrument(name = "change_account_activation_status", skip_all)]
pub async fn change_account_activation_status(
    profile: Profile,
    account_id: Uuid,
    is_active: bool,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::UserManager.to_string()
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

    if target_account_id == profile.acc_id {
        return use_case_err(format!(
            "Prohibited operation. Account ID ({account_id}) could not be 
{target_account_id}."
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
        match is_active {
            true => VerboseStatus::Active,
            false => VerboseStatus::Inactive,
        },
    )
    .await?;

    account_updating_repo.update(updated_account).await
}
