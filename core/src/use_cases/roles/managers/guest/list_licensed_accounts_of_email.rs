use crate::domain::{
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile},
    },
    entities::LicensedResourcesFetching,
};

use clean_base::{
    entities::default_response::FetchManyResponseKind,
    utils::errors::{use_case_err, MappedErrors},
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

    if !profile.is_manager {
        return Err(use_case_err(
            "The current user has no sufficient privileges to guest accounts."
                .to_string(),
            Some(true),
            None,
        ));
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch subscriptions from email
    // ? -----------------------------------------------------------------------

    licensed_resources_fetching_repo.list(email).await
}
