use crate::domain::{
    dtos::{profile::Profile, token::PublicConnectionStringInfo},
    entities::TokenFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List my connection strings
///
/// This function lists all connection strings for the current user.
///
#[tracing::instrument(
    name = "list_my_connection_strings",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_my_connection_strings(
    profile: Profile,
    token_fetching_repo: Box<&dyn TokenFetching>,
) -> Result<FetchManyResponseKind<PublicConnectionStringInfo>, MappedErrors> {
    token_fetching_repo
        .list_connection_strings_by_account_id(profile.acc_id)
        .await
}
