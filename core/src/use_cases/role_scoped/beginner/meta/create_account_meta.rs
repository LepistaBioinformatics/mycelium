use std::collections::HashMap;

use crate::domain::{
    dtos::{account::AccountMetaKey, profile::Profile},
    entities::AccountRegistration,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "create_account_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, account_registration_repo)
)]
pub async fn create_account_meta(
    profile: Profile,
    key: AccountMetaKey,
    value: String,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    account_registration_repo
        .register_account_meta(profile.acc_id, key, value)
        .await
}
