use crate::domain::{
    dtos::{profile::Profile, tenant::Tenant},
    entities::TenantRegistration,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "include_tenant_owner",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_registration_repo))]
pub async fn include_tenant_owner(
    profile: Profile,
    tenant_id: Uuid,
    owner_id: Uuid,
    tenant_registration_repo: Box<&dyn TenantRegistration>,
) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Delete owner
    // ? -----------------------------------------------------------------------

    tenant_registration_repo
        .register_owner(tenant_id, owner_id)
        .await
}
