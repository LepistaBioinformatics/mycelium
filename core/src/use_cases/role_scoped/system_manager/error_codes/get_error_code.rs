use crate::{
    domain::{
        actors::SystemActor,
        dtos::{error_code::ErrorCode, profile::Profile},
        entities::ErrorCodeFetching,
    },
    use_cases::shared::error_codes::find_native_error_code,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// Get details of a single error code
///
/// A native error code is served from code without touching the database;
/// native rows stored in the database are ignored. This action should be only
/// performed by manager or staff users.
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

    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Get error code
    // ? -----------------------------------------------------------------------

    if let Some(native) = find_native_error_code(&prefix, code)? {
        return Ok(FetchResponseKind::Found(native));
    }

    let stored = error_code_fetching_repo.get(prefix, code).await?;

    Ok(ignore_stored_native(stored))
}

/// Native rows stored in the database are ignored: the code is the source
/// of truth for natives
fn ignore_stored_native(
    stored: FetchResponseKind<ErrorCode, (String, i32)>,
) -> FetchResponseKind<ErrorCode, (String, i32)> {
    match stored {
        FetchResponseKind::Found(record) if record.is_native => {
            FetchResponseKind::NotFound(Some((
                record.prefix,
                record.error_number,
            )))
        }
        other => other,
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entities::MockErrorCodeFetching,
        use_cases::role_scoped::system_manager::error_codes::test_profiles::system_manager_profile,
    };

    fn stored(prefix: &str, number: i32, is_native: bool) -> ErrorCode {
        ErrorCode {
            prefix: prefix.to_string(),
            error_number: number,
            code: None,
            message: "Stored failure".to_string(),
            details: None,
            is_internal: false,
            is_native,
        }
    }

    fn repo_returning(record: ErrorCode) -> MockErrorCodeFetching {
        let mut mock = MockErrorCodeFetching::new();

        mock.expect_get().times(1).returning(move |_, _| {
            Ok(FetchResponseKind::Found(record.clone()))
        });

        mock
    }

    #[tokio::test]
    async fn serves_native_without_database() -> Result<(), MappedErrors> {
        let mock = MockErrorCodeFetching::new();

        let response = get_error_code(
            system_manager_profile(),
            "MYC".to_string(),
            2,
            Box::new(&mock),
        )
        .await?;

        let FetchResponseKind::Found(native) = response else {
            panic!("expected the native error code");
        };

        assert!(native.is_native);
        assert_eq!(native.error_number, 2);

        Ok(())
    }

    #[tokio::test]
    async fn serves_custom_from_database() -> Result<(), MappedErrors> {
        let mock = repo_returning(stored("SVC", 1, false));

        let response = get_error_code(
            system_manager_profile(),
            "SVC".to_string(),
            1,
            Box::new(&mock),
        )
        .await?;

        assert!(matches!(response, FetchResponseKind::Found(_)));

        Ok(())
    }

    #[tokio::test]
    async fn ignores_stored_native_rows() -> Result<(), MappedErrors> {
        let mock = repo_returning(stored("MYC", 999, true));

        let response = get_error_code(
            system_manager_profile(),
            "MYC".to_string(),
            999,
            Box::new(&mock),
        )
        .await?;

        assert!(matches!(response, FetchResponseKind::NotFound(_)));

        Ok(())
    }
}
