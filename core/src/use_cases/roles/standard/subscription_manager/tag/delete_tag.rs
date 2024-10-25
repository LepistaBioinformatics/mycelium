use crate::domain::{
    actors::ActorName, dtos::profile::Profile, entities::AccountTagDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tag",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn delete_tag(
    profile: Profile,
    tag_id: Uuid,
    tag_deletion_repo: Box<&dyn AccountTagDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    //
    // Despite the action itself is a deletion one, user must have the
    // permission to update the guest account.
    //
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![
        ActorName::TenantOwner.to_string(),
        ActorName::TenantManager.to_string(),
        ActorName::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_deletion_repo.delete(tag_id).await
}
