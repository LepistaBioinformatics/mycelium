use crate::domain::entities::UserDeletion;

use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::error;
use uuid::Uuid;

#[tracing::instrument(name = "delete_default_user", skip_all)]
pub(super) async fn delete_default_user(
    user_id: Uuid,
    user_deletion_repo: Box<&dyn UserDeletion>,
) -> Result<(), MappedErrors> {
    match user_deletion_repo.delete(user_id).await? {
        DeletionResponseKind::Deleted => Ok(()),
        DeletionResponseKind::NotDeleted(id, msg) => {
            error!("Unable to delete user: {}. Error: {}", id.to_string(), msg);

            use_case_err(format!(
                "Unable to delete user: {}. Error: {}",
                id.to_string(),
                msg
            ))
            .as_error()
        }
    }
}
