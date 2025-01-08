use super::try_to_reach_desired_status::try_to_reach_desired_status;
use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountType,
        profile::Profile,
    },
    entities::{AccountFetching, AccountUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Approve new created accounts.
///
/// This action is needed when a new account is created but not approved by a
/// system administrator. Only checked accounts could perform actions over the
/// system.
#[tracing::instrument(name = "change_account_approval_status", skip_all)]
pub async fn change_account_approval_status(
    profile: Profile,
    account_id: Uuid,
    is_approved: bool,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .with_standard_accounts_access()
        .with_read_write_access()
        .with_roles(vec![SystemActor::UsersManager])
        .get_related_account_or_error()?;

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
        AccountType::Staff => {
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
        match is_approved {
            true => VerboseStatus::Verified,
            false => VerboseStatus::Archived,
        },
    )
    .await?;

    account_updating_repo.update(updated_account).await
}
