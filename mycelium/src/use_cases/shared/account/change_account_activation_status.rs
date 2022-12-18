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

/// Change activation status of the target account.
pub async fn change_account_activation_status(
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

    if (!profile.is_manager) || (target_account_id != profile.account_id) {
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

    account.is_active = !account.is_active;

    account_updating_repo.update(account).await
}
