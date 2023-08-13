use crate::domain::{dtos::profile::Profile, entities::WebHookDeletion};

use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

pub async fn delete_webhook(
    profile: Profile,
    hook_id: Uuid,
    webhook_deletion_repo: Box<&dyn WebHookDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    profile.has_admin_privileges_or_error()?;
    webhook_deletion_repo.delete(hook_id).await
}
