use crate::domain::{
    dtos::{profile::Profile, tenant::Tenant},
    entities::TenantFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

#[tracing::instrument(name = "fetch_tenant_public_info", skip_all)]
pub async fn fetch_tenant_public_info(
    profile: Profile,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<FetchResponseKind<Tenant, Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.on_tenant(tenant_id).get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch the tenant public info
    // ? -----------------------------------------------------------------------

    match tenant_fetching_repo
        .get_tenant_public_by_id(tenant_id)
        .await?
    {
        FetchResponseKind::Found(tenant) => {
            Ok(FetchResponseKind::Found(tenant))
        }
        FetchResponseKind::NotFound(_) => {
            Ok(FetchResponseKind::NotFound(Some(tenant_id)))
        }
    }
}
