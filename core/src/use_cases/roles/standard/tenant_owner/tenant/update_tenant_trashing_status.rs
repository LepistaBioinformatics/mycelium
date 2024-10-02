use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, tenant::Tenant},
    entities::TenantUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_trashing_status", 
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_trashing_status(
    profile: Profile,
    tenant_id: Uuid,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::TenantOwner.to_string()
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_trashing_status(tenant_id, profile.profile_string())
        .await
}
