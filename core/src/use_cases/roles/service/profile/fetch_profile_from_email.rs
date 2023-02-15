use crate::domain::{
    dtos::{email::Email, profile::Profile},
    entities::{LicensedResourcesFetching, ProfileFetching},
};

use clean_base::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use futures::future;
use log::debug;
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

    debug!("Pre-Profile: {:?}", profile);
    debug!("Pre-Licenses: {:?}", licenses);

    // ? -----------------------------------------------------------------------
    // ? Validate profile response
    // ? -----------------------------------------------------------------------

    let mut profile = match profile {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(_) => {
                return Ok(ProfileResponse::UnregisteredUser(email))
            }
            FetchResponseKind::Found(profile) => profile,
        },
    };

    debug!("Parsed Profile from Email: {:?}", profile);

    // ? -----------------------------------------------------------------------
    // ? Validate guests response
    // ? -----------------------------------------------------------------------

    let licenses = match licenses {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => None,
            FetchManyResponseKind::Found(records) => Some(records),
        },
    };

    debug!("Parsed Licenses Record from Email: {:?}", licenses);

    // ? -----------------------------------------------------------------------
    // ? Update profile response to include guests
    // ? -----------------------------------------------------------------------

    profile.licensed_resources = licenses;

    debug!("Build profile: {:?}", profile);

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ProfileResponse::RegisteredUser(profile))
}
