use crate::domain::{dtos::profile::Profile, entities::ErrorCodeDeletion};

use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// Delete a single error code
///
/// This action should be only performed by manager or staff users.
pub async fn delete_error_code(
    profile: Profile,
    prefix: String,
    code: i32,
    error_code_deletion_repo: Box<&dyn ErrorCodeDeletion>,
) -> Result<(String, i32), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to delete error"
                .to_string(),
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Try to delete error code
    // ? -----------------------------------------------------------------------

    match error_code_deletion_repo
        .delete(prefix.to_owned(), code)
        .await?
    {
        DeletionResponseKind::Deleted => Ok((prefix, code)),
        DeletionResponseKind::NotDeleted(_, msg) => {
            return use_case_err(msg).as_error()
        }
    }
}
