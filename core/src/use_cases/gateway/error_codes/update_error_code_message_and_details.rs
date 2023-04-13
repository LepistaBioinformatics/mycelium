use crate::domain::{
    dtos::{error_code::ErrorCode, profile::Profile},
    entities::{ErrorCodeFetching, ErrorCodeUpdating},
};

use clean_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// Update the message and details of an error code.
///
/// This action should be only performed by any registered and active user.
pub async fn update_error_code_message_and_details(
    profile: Profile,
    prefix: String,
    code: i32,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
    error_code_updating_repo: Box<&dyn ErrorCodeUpdating>,
) -> Result<ErrorCode, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register error"
                .to_string(),
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch error code
    // ? -----------------------------------------------------------------------

    let error_code = match error_code_fetching_repo
        .get(prefix.to_owned(), code)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to match errors with prefix {} and code {}",
                prefix, code
            ))
            .as_error();
        }
        FetchResponseKind::Found(error_code) => error_code,
    };

    // ? -----------------------------------------------------------------------
    // ? Perform error update
    // ? -----------------------------------------------------------------------

    match error_code_updating_repo.update(error_code).await? {
        UpdatingResponseKind::NotUpdated(_, msg) => {
            return use_case_err(msg).as_error();
        }
        UpdatingResponseKind::Updated(error_code) => Ok(error_code),
    }
}
