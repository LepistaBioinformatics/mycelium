use crate::domain::{
    dtos::{account::AccountMetaKey, profile::Profile},
    entities::AccountDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_account_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, account_deletion_repo)
)]
pub async fn delete_account_meta(
    profile: Profile,
    key: AccountMetaKey,
    account_deletion_repo: Box<&dyn AccountDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    account_deletion_repo
        .delete_account_meta(profile.acc_id, key)
        .await
}
