use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, tenant::Tenant},
    entities::TenantFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

#[tracing::instrument(
    name = "get_tenant_details",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn get_tenant_details(
    profile: Profile,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    let tenant_related_ids = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch the tenant details
    // ? -----------------------------------------------------------------------

    tenant_fetching_repo
        .get_tenants_by_manager_account(tenant_id, tenant_related_ids)
        .await
}
