use crate::domain::{
    actors::SystemActor,
    dtos::{error_code::ErrorCode, profile::Profile},
    entities::ErrorCodeFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List all error codes
///
/// This function should be used to list all error codes in the application.
/// This action should be only performed by any registered and active user.
#[tracing::instrument(
    name = "list_error_codes",
    skip(profile, page_size, skip, error_code_fetching_repo)
)]
pub async fn list_error_codes(
    profile: Profile,
    prefix: Option<String>,
    code: Option<i32>,
    is_internal: Option<bool>,
    page_size: Option<i32>,
    skip: Option<i32>,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
) -> Result<FetchManyResponseKind<ErrorCode>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_standard_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? List data repository error codes
    // ? -----------------------------------------------------------------------

    error_code_fetching_repo
        .list(prefix, code, is_internal, page_size, skip)
        .await
}
