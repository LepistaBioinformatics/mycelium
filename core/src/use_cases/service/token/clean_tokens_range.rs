use clean_base::{
    entities::default_response::DeletionManyResponseKind,
    utils::errors::MappedErrors,
};

use crate::domain::entities::TokenCleanup;

/// Clean tokens database.
///
/// Remove old token records to reduce the exploitation interface. This function
/// should be used into a cron-job like service.
pub async fn clean_tokens_range(
    token_cleanup_repo: Box<&dyn TokenCleanup>,
) -> Result<DeletionManyResponseKind<Vec<String>>, MappedErrors> {
    token_cleanup_repo.clean(Some(true)).await
}
