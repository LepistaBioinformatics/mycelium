use crate::domain::{
    actors::ActorName,
    dtos::{profile::Profile, tenant::Tenant},
    entities::TenantUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_archiving_status", 
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_archiving_status(
    profile: Profile,
    tenant_id: Uuid,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_update_or_error(vec![
            ActorName::TenantOwner,
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_archiving_status(tenant_id, profile.profile_string())
        .await
}
