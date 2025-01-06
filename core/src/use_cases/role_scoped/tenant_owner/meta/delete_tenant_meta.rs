use crate::domain::{
    dtos::{profile::Profile, tenant::TenantMetaKey},
    entities::{TenantDeletion, TenantFetching},
};

use mycelium_base::{
    entities::{DeletionResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, tenant_fetching_repo, tenant_deletion_repo)
)]
pub async fn delete_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    tenant_deletion_repo: Box<&dyn TenantDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
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

    tenant_deletion_repo
        .delete_tenant_meta(tenant_id, key)
        .await
}
