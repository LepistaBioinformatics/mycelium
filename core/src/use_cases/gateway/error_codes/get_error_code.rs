use crate::domain::{
    dtos::{
        account::VerboseStatus::Active, error_code::ErrorCode,
        native_error_codes::NativeErrorCodes, profile::Profile,
    },
    entities::ErrorCodeFetching,
};

use clean_base::{
    entities::FetchResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// Get details of a single error code
///
/// This action should be only performed by manager or staff users.
pub async fn get_error_code(
    profile: Profile,
    prefix: String,
    code: i32,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
) -> Result<FetchResponseKind<ErrorCode, (String, i32)>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    match profile.verbose_status {
        None => {
            return use_case_err(
                "Unexpected error on check user status".to_string(),
            )
            .with_code(NativeErrorCodes::MYC00004.to_string())
            .as_error()
        }
        Some(status) => {
            if status != Active {
                return use_case_err(
                    "Only active users should perform this action".to_string(),
                )
                .with_code(NativeErrorCodes::MYC00005.to_string())
                .as_error();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Get error code
    // ? -----------------------------------------------------------------------

    error_code_fetching_repo.get(prefix, code).await
}
