use crate::domain::{
    actors::DefaultActor, dtos::profile::Profile, entities::TagDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(name = "delete_tag", skip_all)]
pub async fn delete_tag(
    profile: Profile,
    tag_id: Uuid,
    tag_deletion_repo: Box<&dyn TagDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_create_ids_or_error(vec![
        DefaultActor::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_deletion_repo.delete(tag_id).await
}
