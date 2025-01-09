use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, tag::Tag},
    entities::AccountTagUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tag", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tag(
    profile: Profile,
    tenant_id: Uuid,
    tag: Tag,
    tag_updating_repo: Box<&dyn AccountTagUpdating>,
) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_updating_repo.update(tag).await
}
