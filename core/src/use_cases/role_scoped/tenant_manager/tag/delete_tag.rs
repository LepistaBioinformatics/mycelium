use crate::domain::{
    actors::SystemActor, dtos::profile::Profile, entities::TenantTagDeletion,
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
    tenant_id: Uuid,
    tag_id: Uuid,
    tag_deletion_repo: Box<&dyn TenantTagDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    //
    // Despite the action itself is a deletion one, user must have the
    // permission to update the guest account.
    //
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .with_standard_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_deletion_repo.delete(tag_id).await
}
