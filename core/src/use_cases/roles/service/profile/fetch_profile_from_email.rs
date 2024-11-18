use crate::domain::{
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile},
        route_type::PermissionedRoles,
    },
    entities::{LicensedResourcesFetching, ProfileFetching},
};

use futures::future;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    RegisteredUser(Profile),
    UnregisteredUser(Email),
}

/// Fetch the user profile from email address.
#[tracing::instrument(name = "fetch_profile_from_email", skip_all)]
pub async fn fetch_profile_from_email(
    email: Email,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let (profile, licenses) = future::join(
        profile_fetching_repo.get(Some(email.to_owned()), None),
        licensed_resources_fetching_repo.list(
            email.to_owned(),
            roles,
            permissioned_roles,
            None,
        ),
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
        FetchManyResponseKind::Found(records) => Some(LicensedResources::Records(records)),
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
