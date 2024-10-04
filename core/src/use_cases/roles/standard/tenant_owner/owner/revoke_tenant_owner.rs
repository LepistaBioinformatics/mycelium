use crate::domain::{
    actors::ActorName,
    dtos::{email::Email, profile::Profile, tenant::Tenant},
    entities::{TenantFetching, TenantUpdating},
};

use mycelium_base::{
    dtos::Children,
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "revoke_tenant_owner", 
    fields(profile_id = %profile.acc_id),
    skip(profile, owner_email, tenant_fetching_repo, tenant_updating_repo)
)]
pub async fn revoke_tenant_owner(
    profile: Profile,
    owner_email: Email,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_update_or_error(vec![
            ActorName::TenantOwner.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Collect user
    // ? -----------------------------------------------------------------------

    let tenant = match tenant_fetching_repo
        .get(tenant_id, related_accounts)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err("Tenant not found".to_string()).as_error();
        }
        FetchResponseKind::Found(tenant) => tenant,
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the owners should be revoked
    // ? -----------------------------------------------------------------------

    match tenant.owners {
        Children::Ids(_) => {
            return use_case_err(
                "Unable to revoke owner. Owner information is insufficient"
                    .to_string(),
            )
            .as_error();
        }
        Children::Records(records) => {
            let emails: Vec<String> =
                records.iter().map(|i| i.email.to_owned()).collect();

            if emails.len() == 1 && emails.contains(&owner_email.get_email()) {
                return use_case_err(
                    "Tenant should contains at last one owner".to_string(),
                )
                .as_error();
            }

            if !records
                .iter()
                .any(|record| record.email == owner_email.get_email())
            {
                return use_case_err(
                    "Informed Owner is not in the tenant".to_string(),
                )
                .as_error();
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Register the owner
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .remove_owner(tenant_id, None, Some(owner_email))
        .await
}
