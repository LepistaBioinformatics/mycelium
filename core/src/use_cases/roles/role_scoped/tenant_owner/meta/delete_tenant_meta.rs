use crate::domain::{
    actors::ActorName,
    dtos::{profile::Profile, tenant::TenantMetaKey},
    entities::TenantDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, tenant_deletion_repo)
)]
pub async fn delete_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    tenant_deletion_repo: Box<&dyn TenantDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_default_write_ids_or_error(vec![ActorName::TenantOwner])?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    tenant_deletion_repo
        .delete_tenant_meta(tenant_id, key)
        .await
}
