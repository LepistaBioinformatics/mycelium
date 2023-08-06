use crate::domain::{
    dtos::{email::Email, profile::Profile},
    entities::{LicensedResourcesFetching, ProfileFetching},
};

use clean_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use futures::future;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    RegisteredUser(Profile),
    UnregisteredUser(Email),
}

/// Fetch the user profile from email address.
pub async fn fetch_profile_from_email(
    email: Email,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let (profile, licenses) = future::join(
        profile_fetching_repo.get(Some(email.to_owned()), None),
        licensed_resources_fetching_repo.list(email.to_owned()),
    )
    .await;

    // ? -----------------------------------------------------------------------
    // ? Validate profile response
    // ? -----------------------------------------------------------------------

    let mut profile = match profile? {
        FetchResponseKind::NotFound(_) => {
            return Ok(ProfileResponse::UnregisteredUser(email))
        }
        FetchResponseKind::Found(profile) => profile,
    };

    // ? -----------------------------------------------------------------------
    // ? Validate guests response
    // ? -----------------------------------------------------------------------

    let licenses = match licenses? {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => Some(records),
        _ => panic!(
            "Paginated results parsing not implemented in `fetch_profile_from_email` use-case."
        ),
    };

    // ? -----------------------------------------------------------------------
    // ? Update profile response to include guests
    // ? -----------------------------------------------------------------------

    profile.licensed_resources = licenses;

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ProfileResponse::RegisteredUser(profile))
}
