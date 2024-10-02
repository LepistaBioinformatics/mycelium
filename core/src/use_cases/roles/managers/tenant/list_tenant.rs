use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

use crate::domain::{
    dtos::{
        profile::Profile,
        tenant::{Tenant, TenantMeta, TenantStatus},
    },
    entities::TenantFetching,
};

#[tracing::instrument(
    name = "list_tenant",
    fields(
        account_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_fetching_repo)
)]
pub async fn list_tenant(
    profile: Profile,
    name: Option<String>,
    owner: Option<Uuid>,
    metadata: Option<TenantMeta>,
    status: Option<TenantStatus>,
    tag_value: Option<String>,
    tag_meta: Option<String>,
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
        .filter(name, owner, metadata, status, tag_value, tag_meta)
        .await
}
