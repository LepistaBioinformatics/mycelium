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

    let tenant_related_ids = match profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_ids_or_error()
    {
        Ok(tenant_related_ids) => tenant_related_ids,
        Err(e) => {
            if !e.expected() {
                return Err(e);
            }

            //
            // If the current account is not a tenant manager, we need to
            // fetch the tenant details from the tenant owner account.
            //
            vec![]
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch the tenant details
    // ? -----------------------------------------------------------------------

    //
    // If the current account is a tenant manager, we need to fetch the tenant
    // details from the tenant manager account.
    //
    if tenant_related_ids.len() > 0 {
        tenant_fetching_repo
            .get_tenants_by_manager_account(tenant_id, tenant_related_ids)
            .await
    //
    // Otherwise, we need to fetch the tenant details from the tenant owner
    // account. This is because the tenant owner account is the one that
    // has the full access to the tenant details.
    //
    } else {
        profile.with_tenant_ownership_or_error(tenant_id)?;

        tenant_fetching_repo
            .get_tenant_owned_by_me(tenant_id, profile.get_owners_ids())
            .await
    }
}
