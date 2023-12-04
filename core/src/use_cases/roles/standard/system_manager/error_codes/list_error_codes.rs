use crate::domain::{
    dtos::{
        account::VerboseStatus::Active, error_code::ErrorCode,
        native_error_codes::NativeErrorCodes::*, profile::Profile,
    },
    entities::ErrorCodeFetching,
};

use clean_base::{
    entities::FetchManyResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// List all error codes
///
/// This function should be used to list all error codes in the application.
/// This action should be only performed by any registered and active user.
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

    match profile.verbose_status {
        None => {
            return use_case_err(
                "Unexpected error on check user status".to_string(),
            )
            .with_code(MYC00004.as_str())
            .as_error()
        }
        Some(status) => {
            if status != Active {
                return use_case_err(
                    "Only active users should perform this action".to_string(),
                )
                .with_code(MYC00005.as_str())
                .as_error();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? List data repository error codes
    // ? -----------------------------------------------------------------------

    error_code_fetching_repo
        .list(prefix, code, is_internal, page_size, skip)
        .await
}
