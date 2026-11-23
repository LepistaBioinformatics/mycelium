use crate::domain::{
    actors::SystemActor,
    dtos::{
        error_code::ErrorCode, error_code_filters::ErrorCodeFilters,
        profile::Profile,
    },
    entities::ErrorCodeFetching,
};
use crate::use_cases::shared::error_codes::list_native_then_custom_error_codes;

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List all error codes
///
/// The in-code native error codes come first, then the custom codes stored
/// in the database, as one paginated result. This action should be only
/// performed by system managers.
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
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? List data repository error codes
    // ? -----------------------------------------------------------------------

    let filters = ErrorCodeFilters {
        prefix,
        code,
        is_internal,
    };

    list_native_then_custom_error_codes(
        filters,
        page_size,
        skip,
        error_code_fetching_repo,
    )
    .await
}
