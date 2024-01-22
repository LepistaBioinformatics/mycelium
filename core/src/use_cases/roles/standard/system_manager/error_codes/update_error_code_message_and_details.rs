use crate::domain::{
    actors::DefaultActor,
    dtos::{
        error_code::ErrorCode, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::{ErrorCodeFetching, ErrorCodeUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

/// Update the message and details of an error code.
///
/// This action should be only performed by any registered and active user.
pub async fn update_error_code_message_and_details(
    profile: Profile,
    prefix: String,
    code: i32,
    message: String,
    details: Option<String>,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
    error_code_updating_repo: Box<&dyn ErrorCodeUpdating>,
) -> Result<ErrorCode, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::SystemManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch error code
    // ? -----------------------------------------------------------------------

    let mut error_code = match error_code_fetching_repo
        .get(prefix.to_owned(), code)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to match errors with prefix {} and code {}",
                prefix, code
            ))
            .with_code(NativeErrorCodes::MYC00006.as_str())
            .as_error();
        }
        FetchResponseKind::Found(error_code) => error_code,
    };

    // ? -----------------------------------------------------------------------
    // ? Perform error update
    // ? -----------------------------------------------------------------------

    error_code.message = message;
    error_code.details = details;

    match error_code_updating_repo.update(error_code).await? {
        UpdatingResponseKind::NotUpdated(_, msg) => {
            return use_case_err(msg)
                .with_code(NativeErrorCodes::MYC00007.as_str())
                .as_error();
        }
        UpdatingResponseKind::Updated(error_code) => Ok(error_code),
    }
}
