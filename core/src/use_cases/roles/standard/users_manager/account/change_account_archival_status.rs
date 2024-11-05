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

/// Change archival status of and account.
///
/// After created new accounts could be approved or archived. Case archived
/// these use-case should be used.
#[tracing::instrument(name = "change_account_archival_status", skip_all)]
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
        match is_archived {
            true => VerboseStatus::Archived,
            false => VerboseStatus::Unverified,
        },
    )
    .await?;

    account_updating_repo.update(updated_account).await
}