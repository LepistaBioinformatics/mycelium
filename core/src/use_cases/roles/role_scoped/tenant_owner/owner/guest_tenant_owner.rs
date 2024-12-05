use crate::domain::{
    actors::ActorName,
    dtos::{email::Email, profile::Profile, user::Provider},
    entities::{TenantOwnerConnection, TenantUpdating, UserFetching},
};

use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
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
) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            ActorName::TenantOwner,
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Collect user
    // ? -----------------------------------------------------------------------

    let user = match owner_fetching_repo.get_user_by_email(owner_email).await? {
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

    if let Some(id) = user.id {
        tenant_updating_repo
            .register_owner(
                tenant_id,
                id,
                format!("account-id:{}", profile.acc_id.to_string()),
            )
            .await
    } else {
        return use_case_err(
            "Unable to guest user to tenant. Used ID is invalid.".to_string(),
        )
        .as_error();
    }
}
