use crate::domain::{
    dtos::{profile::Profile, tenant::Tenant},
    entities::{TenantFetching, TenantUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_name_and_description", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_name_and_description(
    profile: Profile,
    tenant_id: Uuid,
    tenant_name: Option<String>,
    tenant_description: Option<String>,
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
        .update_name_and_description(tenant_id, tenant_name, tenant_description)
        .await
}
