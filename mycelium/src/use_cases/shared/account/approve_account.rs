use crate::domain::{
    dtos::{account::AccountDTO, profile::ProfileDTO},
    entities::shared::{
        account_fetching::AccountFetching, account_updating::AccountUpdating,
    },
};

use clean_base::{
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
    profile: ProfileDTO,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<AccountDTO>, MappedErrors> {
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

    let target_account_id = match account.id {
        None => {
            return Err(use_case_err(
                format!("Prohibited operation."),
                Some(true),
                None,
            ))
        }
        Some(res) => res,
    };

    if (!profile.is_manager) || (target_account_id != profile.account) {
        return Err(use_case_err(
            format!(
                "Not enough permissions deactivate the account {:?}.",
                target_account_id
            ),
            Some(true),
            None,
        ));
    }

    // ? -----------------------------------------------------------------------
    // ? Update account status
    // ? -----------------------------------------------------------------------

    account.is_checked = true;

    account_updating_repo.update(account).await
}
