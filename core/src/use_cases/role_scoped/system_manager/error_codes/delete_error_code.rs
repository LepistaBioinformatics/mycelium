use crate::domain::{
    actors::SystemActor, dtos::profile::Profile, entities::ErrorCodeDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Delete a single error code
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

    profile.get_default_write_ids_or_error(vec![
        SystemActor::SystemManager.to_string()
    ])?;

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
