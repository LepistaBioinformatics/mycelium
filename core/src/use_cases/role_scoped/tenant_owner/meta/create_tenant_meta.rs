use crate::domain::{
    dtos::{profile::Profile, tenant::TenantMetaKey},
    entities::TenantRegistration,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tracing::instrument(
    name = "create_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, tenant_registration_repo)
)]
pub async fn create_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    value: String,
    tenant_registration_repo: Box<&dyn TenantRegistration>,
) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    tenant_registration_repo
        .register_tenant_meta(profile.get_owners_ids(), tenant_id, key, value)
        .await
}
