use crate::domain::{
    actors::DefaultActor,
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile},
    },
    entities::LicensedResourcesFetching,
};

use clean_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// Get all licenses related to email
///
/// Fetch all subscription accounts which an email was guest.
pub async fn list_licensed_accounts_of_email(
    profile: Profile,
    email: Email,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<FetchManyResponseKind<LicensedResources>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .get_view_ids_or_error(vec![DefaultActor::GuestManager.to_string()])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch subscriptions from email
    // ? -----------------------------------------------------------------------

    licensed_resources_fetching_repo.list(email).await
}
