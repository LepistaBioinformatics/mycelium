use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, tag::Tag},
    entities::TenantTagUpdating,
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
    tag_updating_repo: Box<&dyn TenantTagUpdating>,
) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![
        SystemActor::TenantOwner.to_string(),
        SystemActor::TenantManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_updating_repo.update(tag).await
}
