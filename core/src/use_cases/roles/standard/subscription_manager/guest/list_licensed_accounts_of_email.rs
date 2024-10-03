use crate::domain::{
    actors::ActorName,
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile},
    },
    entities::LicensedResourcesFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Get all licenses related to email
///
/// Fetch all subscription accounts which an email was guest.
#[tracing::instrument(
    name = "list_licensed_accounts_of_email",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_licensed_accounts_of_email(
    profile: Profile,
    tenant_id: Uuid,
    email: Email,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<FetchManyResponseKind<LicensedResources>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_view_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch subscriptions from email
    // ? -----------------------------------------------------------------------

    licensed_resources_fetching_repo
        .list(email, Some(related_accounts))
        .await
}
