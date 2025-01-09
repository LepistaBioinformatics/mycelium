use crate::domain::{
    dtos::{
        account::{AccountMeta, AccountMetaKey},
        profile::Profile,
    },
    entities::AccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "update_account_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, account_updating_repo)
)]
pub async fn update_account_meta(
    profile: Profile,
    key: AccountMetaKey,
    value: String,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<AccountMeta>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    account_updating_repo
        .update_account_meta(profile.acc_id, key, value)
        .await
}
