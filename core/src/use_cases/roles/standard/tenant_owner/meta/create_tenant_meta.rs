use crate::domain::{
    actors::ActorName,
    dtos::{
        profile::Profile,
        tenant::{TenantMeta, TenantMetaKey},
    },
    entities::TenantRegistration,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
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
) -> Result<CreateResponseKind<TenantMeta>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_default_create_ids_or_error(vec![
            ActorName::TenantOwner.to_string()
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    tenant_registration_repo
        .register_tenant_meta(tenant_id, key, value)
        .await
}
