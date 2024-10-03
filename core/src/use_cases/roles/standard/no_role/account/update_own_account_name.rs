use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::AccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};

/// Update the own account.
///
/// This function uses the id of the Profile to fetch and update the account
/// name, allowing only the account owner to update the account name.
#[tracing::instrument(name = "update_own_account_name", skip_all)]
pub async fn update_own_account_name(
    profile: Profile,
    name: String,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account_updating_repo
        .update_own_account_name(profile.acc_id, name)
        .await
}
