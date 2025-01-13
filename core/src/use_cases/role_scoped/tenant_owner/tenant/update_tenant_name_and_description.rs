use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes, profile::Profile, tenant::Tenant,
    },
    entities::{TenantFetching, TenantUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{fetching_err, MappedErrors},
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
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    let mut tenant = match tenant_fetching_repo
        .get_tenant_owned_by_me(tenant_id, profile.get_owners_ids())
        .await?
    {
        FetchResponseKind::Found(tenant) => tenant,
        FetchResponseKind::NotFound(_) => {
            return fetching_err("Tenant not found")
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    if let Some(name) = tenant_name {
        tenant.name = name;
    }

    if let Some(description) = tenant_description {
        tenant.description = Some(description);
    }

    tenant_updating_repo
        .update_name_and_description(tenant_id, tenant)
        .await
}
