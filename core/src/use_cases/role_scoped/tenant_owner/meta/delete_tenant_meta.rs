use crate::domain::{
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
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    tenant_deletion_repo
        .delete_tenant_meta(tenant_id, key)
        .await
}
