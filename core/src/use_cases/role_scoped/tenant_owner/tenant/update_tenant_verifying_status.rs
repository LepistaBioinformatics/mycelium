use crate::domain::{
    dtos::{
        profile::Profile,
        tenant::{Tenant, TenantStatus},
    },
    entities::{TenantFetching, TenantUpdating},
};

use chrono::Local;
use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_verifying_status", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_verifying_status(
    profile: Profile,
    tenant_id: Uuid,
    verified: bool,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Collect user
    // ? -----------------------------------------------------------------------

    match tenant_fetching_repo
        .get_tenant_owned_by_me(
            tenant_id,
            profile.owners.iter().map(|o| o.id).collect(),
        )
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err("Tenant not found".to_string()).as_error();
        }
        FetchResponseKind::Found(tenant) => tenant,
    };

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_status(
            tenant_id,
            TenantStatus::Verified {
                verified,
                at: Local::now(),
                by: profile.profile_string(),
            },
        )
        .await
}
