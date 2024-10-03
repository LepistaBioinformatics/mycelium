use crate::domain::{
    actors::DefaultActor,
    dtos::{
        profile::Profile, related_accounts::RelatedAccounts, tenant::Tenant,
    },
    entities::TenantUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_name_and_description", 
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_name_and_description(
    profile: Profile,
    tenant_id: Uuid,
    tenant_name: Option<String>,
    tenant_description: Option<String>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    if let RelatedAccounts::AllowedAccounts(allowed_ids) = &profile
        .get_related_account_with_default_update_or_error(vec![
            DefaultActor::TenantOwner.to_string(),
        ])?
    {
        if !allowed_ids.contains(&tenant_id) {
            return use_case_err(
                "User is not allowed to perform this action".to_string(),
            )
            .as_error();
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_name_and_description(tenant_id, tenant_name, tenant_description)
        .await
}
