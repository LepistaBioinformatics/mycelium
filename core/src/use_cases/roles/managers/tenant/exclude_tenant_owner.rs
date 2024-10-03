use crate::domain::{dtos::profile::Profile, entities::TenantDeletion};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "exclude_tenant_owner",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_deletion_repo))]
pub async fn exclude_tenant_owner(
    profile: Profile,
    tenant_id: Uuid,
    owner_id: Uuid,
    tenant_deletion_repo: Box<&dyn TenantDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Delete owner
    // ? -----------------------------------------------------------------------

    tenant_deletion_repo.delete_owner(tenant_id, owner_id).await
}
