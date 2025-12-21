use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes, profile::Profile, tenant::Tenant,
    },
    entities::TenantFetching,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(name = "fetch_tenant_public_info", skip_all)]
pub async fn fetch_tenant_public_info(
    profile: Profile,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<FetchResponseKind<Tenant, Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile has tenant access
    // ? -----------------------------------------------------------------------

    let has_tenant_license =
        profile.on_tenant(tenant_id).get_ids_or_error().is_ok();

    let has_tenant_ownership =
        profile.with_tenant_ownership_or_error(tenant_id).is_ok();

    if ![has_tenant_license, has_tenant_ownership]
        .iter()
        .any(|&x| x)
    {
        return use_case_err("User does not have access to the tenant")
            .with_code(NativeErrorCodes::MYC00019)
            .with_exp_true()
            .as_error();
    }

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
