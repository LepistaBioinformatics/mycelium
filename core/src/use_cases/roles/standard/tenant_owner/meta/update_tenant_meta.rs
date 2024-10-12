use crate::domain::{
    actors::ActorName,
    dtos::{
        profile::Profile,
        tenant::{TenantMeta, TenantMetaKey},
    },
    entities::TenantUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, tenant_updating_repo)
)]
pub async fn update_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    value: String,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<TenantMeta>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_default_update_ids_or_error(vec![ActorName::TenantOwner])?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_meta(tenant_id, key, value)
        .await
}
