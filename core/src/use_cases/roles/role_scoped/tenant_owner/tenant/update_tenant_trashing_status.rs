use crate::domain::{
    actors::ActorName,
    dtos::{
        profile::Profile,
        tenant::{Tenant, TenantStatus},
    },
    entities::TenantUpdating,
};

use chrono::Local;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_trashing_status", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_trashing_status(
    profile: Profile,
    tenant_id: Uuid,
    trashed: bool,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            ActorName::TenantOwner,
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_status(
            tenant_id,
            TenantStatus::Trashed {
                trashed,
                at: Local::now(),
                by: profile.profile_string(),
            },
        )
        .await
}
