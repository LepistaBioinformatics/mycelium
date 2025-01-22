use super::update_tenant_status::update_tenant_status;
use crate::domain::{
    dtos::{
        profile::Profile,
        tenant::{Tenant, TenantStatus},
    },
    entities::{TenantFetching, TenantUpdating},
};

use chrono::Local;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_archiving_status", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_archiving_status(
    profile: Profile,
    tenant_id: Uuid,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    update_tenant_status(
        profile.to_owned(),
        TenantStatus::Archived {
            at: Local::now(),
            by: profile.profile_string(),
        },
        tenant_id,
        tenant_updating_repo,
        tenant_fetching_repo,
    )
    .await
}
