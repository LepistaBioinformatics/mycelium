use crate::domain::{
    actors::ActorName,
    dtos::{profile::Profile, tag::Tag},
    entities::AccountTagUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "update_tag", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tag(
    profile: Profile,
    tag: Tag,
    tag_updating_repo: Box<&dyn AccountTagUpdating>,
) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![
        ActorName::TenantOwner.to_string(),
        ActorName::TenantManager.to_string(),
        ActorName::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_updating_repo.update(tag).await
}
