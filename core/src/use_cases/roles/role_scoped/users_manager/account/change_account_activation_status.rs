use super::try_to_reach_desired_status::try_to_reach_desired_status;
use crate::domain::{
    actors::ActorName,
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountTypeV2,
        profile::Profile,
    },
    entities::{AccountFetching, AccountUpdating},
};

use mycelium_base::{
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

    let related_accounts = profile
        .get_related_account_with_default_write_or_error(vec![
            ActorName::UsersManager,
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch target account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo
        .get(account_id, related_accounts)
        .await?
    {
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

    match account.to_owned().account_type {
        AccountTypeV2::Staff => {
            if profile.is_manager && !profile.is_staff {
                return use_case_err(String::from(
                    "Prohibited operation. Managers could not perform editions 
on accounts with more privileges than himself.",
                ))
                .as_error();
            }
        }
        _ => {
            return use_case_err(format!(
                "Prohibited operation. Invalid account type"
            ))
            .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update account status
    // ? -----------------------------------------------------------------------

    let updated_account = try_to_reach_desired_status(
        account.to_owned(),
        match is_active {
            true => VerboseStatus::Verified,
            false => VerboseStatus::Inactive,
        },
    )
    .await?;

    account_updating_repo.update(updated_account).await
}
