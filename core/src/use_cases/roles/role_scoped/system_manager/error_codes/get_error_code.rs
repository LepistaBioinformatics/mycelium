use crate::domain::{
    actors::SystemActor,
    dtos::{error_code::ErrorCode, profile::Profile},
    entities::ErrorCodeFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// Get details of a single error code
///
/// This action should be only performed by manager or staff users.
#[tracing::instrument(
    name = "get_error_code",
    skip(profile, error_code_fetching_repo)
)]
pub async fn get_error_code(
    profile: Profile,
    prefix: String,
    code: i32,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
) -> Result<FetchResponseKind<ErrorCode, (String, i32)>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_read_ids_or_error(vec![
        SystemActor::SystemManager.to_string()
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Get error code
    // ? -----------------------------------------------------------------------

    error_code_fetching_repo.get(prefix, code).await
}
