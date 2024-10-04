use crate::domain::{
    actors::ActorName,
    dtos::{email::Email, profile::Profile, tenant::Tenant, user::Provider},
    entities::{TenantUpdating, UserFetching},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "guest_tenant_owner", 
    fields(profile_id = %profile.acc_id),
    skip(profile, owner_email, owner_fetching_repo, tenant_updating_repo)
)]
pub async fn guest_tenant_owner(
    profile: Profile,
    owner_email: Email,
    tenant_id: Uuid,
    owner_fetching_repo: Box<&dyn UserFetching>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_update_or_error(vec![
            ActorName::TenantOwner.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Collect user
    // ? -----------------------------------------------------------------------

    let user = match owner_fetching_repo
        .get(None, Some(owner_email), None)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err("User not found".to_string()).as_error();
        }
        FetchResponseKind::Found(user) => user,
    };

    if let Some(Provider::Internal(_)) = user.provider() {
        if !user.is_active {
            return use_case_err("User is not active".to_string()).as_error();
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Register the owner
    // ? -----------------------------------------------------------------------

    match user.id {
        Some(id) => tenant_updating_repo.register_owner(tenant_id, id).await,
        None => {
            return use_case_err(
                "Unable to guest user to tenant. Used ID is invalid."
                    .to_string(),
            )
            .as_error();
        }
    }
}
