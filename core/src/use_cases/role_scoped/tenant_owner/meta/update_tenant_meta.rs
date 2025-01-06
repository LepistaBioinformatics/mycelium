use crate::domain::{
    dtos::{
        profile::Profile,
        tenant::{TenantMeta, TenantMetaKey},
    },
    entities::{TenantFetching, TenantUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, tenant_fetching_repo, tenant_updating_repo)
)]
pub async fn update_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    value: String,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<TenantMeta>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    match tenant_fetching_repo
        .get_tenant_owned_by_me(tenant_id, profile.get_owners_ids())
        .await?
    {
        FetchResponseKind::NotFound(msg) => {
            return use_case_err(
                msg.unwrap_or(
                    "Tenant does not exist or inaccessible for the user"
                        .to_string(),
                ),
            )
            .as_error()
        }
        FetchResponseKind::Found(tenant) => tenant,
    };

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_meta(tenant_id, key, value)
        .await
}
