use crate::domain::{
    actors::ActorName, dtos::profile::Profile, entities::WebHookDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_webhook",
    skip(profile, webhook_deletion_repo)
)]
pub async fn delete_webhook(
    profile: Profile,
    hook_id: Uuid,
    webhook_deletion_repo: Box<&dyn WebHookDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![ActorName::SystemManager])?;

    // ? -----------------------------------------------------------------------
    // ? Delete webhook
    // ? -----------------------------------------------------------------------

    webhook_deletion_repo.delete(hook_id).await
}
