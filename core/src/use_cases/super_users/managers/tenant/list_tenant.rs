use crate::domain::{
    dtos::{
        profile::Profile,
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "list_tenant",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_fetching_repo)
)]
pub async fn list_tenant(
    profile: Profile,
    name: Option<String>,
    owner: Option<Uuid>,
    metadata: Option<(TenantMetaKey, String)>,
    tag: Option<(String, String)>,
    page_size: Option<i32>,
    skip: Option<i32>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Filter Tenants
    // ? -----------------------------------------------------------------------

    tenant_fetching_repo
        .filter_tenants_as_manager(name, owner, metadata, tag, page_size, skip)
        .await
}
