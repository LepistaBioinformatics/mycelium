use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        tenant::{Tenant, TenantStatus},
    },
    entities::{TenantFetching, TenantUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_status", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub(super) async fn update_tenant_status(
    profile: Profile,
    next_status: TenantStatus,
    tenant_id: Uuid,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    let tenant = match tenant_fetching_repo
        .get_tenant_owned_by_me(tenant_id, profile.get_owners_ids())
        .await?
    {
        FetchResponseKind::Found(tenant) => tenant,
        FetchResponseKind::NotFound(_) => {
            return fetching_err("Tenant not found")
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the current status is the same as the next status
    // ? -----------------------------------------------------------------------

    if let Some(mut status) = tenant.status.clone() {
        //
        // Sort statuses by date to select the last status
        //
        status.sort_by_key(|s| match s {
            TenantStatus::Archived { at, .. } => *at,
            TenantStatus::Trashed { at, .. } => *at,
            TenantStatus::Verified { at, .. } => *at,
        });

        if let Some(last_status) = status.last() {
            let is_the_same = match next_status {
                TenantStatus::Verified { .. } => last_status.is_verified(),
                TenantStatus::Trashed { .. } => last_status.is_trashed(),
                TenantStatus::Archived { .. } => last_status.is_archived(),
            };

            if is_the_same {
                return fetching_err(
                    "Tenant status is already the same as the next status",
                )
                .with_code(NativeErrorCodes::MYC00018)
                .with_exp_true()
                .as_error();
            }
        };
    }

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    tenant_updating_repo
        .update_tenant_status(tenant_id, next_status)
        .await
}
