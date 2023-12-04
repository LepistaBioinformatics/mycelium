use crate::domain::{
    actors::DefaultActor, dtos::profile::Profile, entities::WebHookDeletion,
};

use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

pub async fn delete_webhook(
    profile: Profile,
    hook_id: Uuid,
    webhook_deletion_repo: Box<&dyn WebHookDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    profile.get_delete_ids_or_error(vec![
        DefaultActor::SystemManager.to_string()
    ])?;

    webhook_deletion_repo.delete(hook_id).await
}
