use crate::domain::{
    dtos::profile::Profile,
    entities::{TenantOwnerConnection, TenantUpdating},
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "include_tenant_owner",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_updating_repo))]
pub async fn include_tenant_owner(
    profile: Profile,
    tenant_id: Uuid,
    owner_id: Uuid,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Delete owner
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .register_owner(
            tenant_id,
            owner_id,
            format!("account-id:{}", profile.acc_id),
        )
        .await
}
