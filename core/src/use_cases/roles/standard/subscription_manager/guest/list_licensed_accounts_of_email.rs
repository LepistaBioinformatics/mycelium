use crate::domain::{
    actors::DefaultActor,
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile},
    },
    entities::LicensedResourcesFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// Get all licenses related to email
///
/// Fetch all subscription accounts which an email was guest.
#[tracing::instrument(
    name = "list_licensed_accounts_of_email",
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn list_licensed_accounts_of_email(
    profile: Profile,
    email: Email,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<FetchManyResponseKind<LicensedResources>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_view_ids_or_error(vec![
        DefaultActor::TenantOwner.to_string(),
        DefaultActor::TenantManager.to_string(),
        DefaultActor::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch subscriptions from email
    // ? -----------------------------------------------------------------------

    licensed_resources_fetching_repo.list(email).await
}
