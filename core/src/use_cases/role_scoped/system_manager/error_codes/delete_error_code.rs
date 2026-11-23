use crate::{
    domain::{
        actors::SystemActor,
        dtos::{native_error_codes::NativeErrorCodes, profile::Profile},
        entities::ErrorCodeDeletion,
    },
    use_cases::shared::error_codes::refuse_native_error_code_write,
};

use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Delete a single error code
///
/// Native error codes are defined in code and cannot be deleted.
///
/// This action should be only performed by manager or staff users.
#[tracing::instrument(
    name = "delete_error_code",
    skip(profile, error_code_deletion_repo)
)]
pub async fn delete_error_code(
    profile: Profile,
    prefix: String,
    code: i32,
    error_code_deletion_repo: Box<&dyn ErrorCodeDeletion>,
) -> Result<(String, i32), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Try to delete error code
    // ? -----------------------------------------------------------------------

    refuse_native_error_code_write(&prefix, code)?;

    match error_code_deletion_repo
        .delete(prefix.to_owned(), code)
        .await?
    {
        DeletionResponseKind::Deleted => Ok((prefix, code)),
        DeletionResponseKind::NotDeleted(_, msg) => {
            return use_case_err(msg)
                .with_code(NativeErrorCodes::MYC00018)
                .with_exp_true()
                .as_error()
        }
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entities::MockErrorCodeDeletion,
        use_cases::role_scoped::system_manager::error_codes::test_profiles::system_manager_profile,
    };

    #[tokio::test]
    async fn refuses_deleting_native() {
        let mock = MockErrorCodeDeletion::new();

        let error = delete_error_code(
            system_manager_profile(),
            "MYC".to_string(),
            5,
            Box::new(&mock),
        )
        .await
        .expect_err("deleting a native must be refused");

        assert!(error.expected());
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }

    #[tokio::test]
    async fn deletes_custom() -> Result<(), MappedErrors> {
        let mut mock = MockErrorCodeDeletion::new();

        mock.expect_delete()
            .times(1)
            .returning(|_, _| Ok(DeletionResponseKind::Deleted));

        let deleted = delete_error_code(
            system_manager_profile(),
            "SVC".to_string(),
            5,
            Box::new(&mock),
        )
        .await?;

        assert_eq!(deleted, ("SVC".to_string(), 5));

        Ok(())
    }
}
