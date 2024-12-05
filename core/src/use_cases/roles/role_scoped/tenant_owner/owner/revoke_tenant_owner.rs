use crate::domain::{
    dtos::{email::Email, profile::Profile},
    entities::{TenantDeletion, TenantFetching},
};

use mycelium_base::{
    dtos::Children,
    entities::{DeletionResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "revoke_tenant_owner", 
    fields(profile_id = %profile.acc_id),
    skip(profile, owner_email, tenant_fetching_repo, tenant_deletion_repo)
)]
pub async fn revoke_tenant_owner(
    profile: Profile,
    owner_email: Email,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    tenant_deletion_repo: Box<&dyn TenantDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Collect user
    // ? -----------------------------------------------------------------------

    let tenant = match tenant_fetching_repo
        .get_tenant_owned_by_me(
            tenant_id,
            profile.owners.iter().map(|o| o.id).collect(),
        )
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

    tenant_deletion_repo
        .delete_owner(tenant_id, None, Some(owner_email))
        .await
}
