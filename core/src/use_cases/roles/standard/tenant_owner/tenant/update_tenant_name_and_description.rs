use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, tenant::Tenant},
    entities::TenantUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "update_tenant_name_and_description", 
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_name_and_description(
    profile: Profile,
    tenant_name: Option<String>,
    tenant_description: Option<String>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::TenantOwner.to_string()
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_name_and_description(tenant_name, tenant_description)
        .await
}
