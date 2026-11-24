use crate::domain::{
    actors::SystemActor,
    dtos::{
        error_code::ErrorCode, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::{ErrorCodeFetching, ErrorCodeUpdating},
};
use crate::use_cases::shared::error_codes::refuse_native_error_code_write;

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

/// Update the message and details of an error code.
///
/// Native error codes are defined in code and cannot be updated.
///
/// This action should be only performed by any registered and active user.
#[tracing::instrument(
    name = "update_error_code_message_and_details",
    skip(
        profile,
        message,
        details,
        error_code_fetching_repo,
        error_code_updating_repo
    )
)]
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

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch error code
    // ? -----------------------------------------------------------------------

    refuse_native_error_code_write(&prefix, code)?;

    let mut error_code = match error_code_fetching_repo
        .get(prefix.to_owned(), code)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to match errors with prefix {} and code {}",
                prefix, code
            ))
            .with_code(NativeErrorCodes::MYC00006)
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
                .with_code(NativeErrorCodes::MYC00007)
                .as_error();
        }
        UpdatingResponseKind::Updated(error_code) => Ok(error_code),
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entities::{MockErrorCodeFetching, MockErrorCodeUpdating},
        use_cases::role_scoped::system_manager::error_codes::test_profiles::system_manager_profile,
    };

    #[tokio::test]
    async fn refuses_updating_native() {
        let fetching = MockErrorCodeFetching::new();
        let updating = MockErrorCodeUpdating::new();

        let error = update_error_code_message_and_details(
            system_manager_profile(),
            "MYC".to_string(),
            5,
            "Changed message".to_string(),
            None,
            Box::new(&fetching),
            Box::new(&updating),
        )
        .await
        .expect_err("updating a native must be refused");

        assert!(error.expected());
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }
}
