use crate::domain::{
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile, TenantsOwnership},
        security_group::PermissionedRole,
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
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    RegisteredUser(Profile),
    UnregisteredUser(Email),
}

/// Fetch the user profile from email address.
#[tracing::instrument(
    name = "fetch_profile_from_email", 
    fields(
        email = %email.redacted_email(),
    ),
    skip(email, profile_fetching_repo, licensed_resources_fetching_repo)
)]
pub async fn fetch_profile_from_email(
    email: Email,
    was_verified: Option<bool>,
    tenant: Option<Uuid>,
    roles: Option<Vec<PermissionedRole>>,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let (profile, licenses, ownership) = future::join3(
        profile_fetching_repo.get_from_email(email.to_owned()),
        licensed_resources_fetching_repo.list_licensed_resources(
            email.to_owned(),
            tenant,
            roles,
            None,
            was_verified,
        ),
        licensed_resources_fetching_repo
            .list_tenants_ownership(email.to_owned(), tenant),
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

    profile.licensed_resources = match licenses? {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => {
            Some(LicensedResources::Records(records))
        }
        _ => panic!(
            "Paginated licenses not implemented when fetch profile from email"
        ),
    };

    // ? -----------------------------------------------------------------------
    // ? Validate ownership response
    // ? -----------------------------------------------------------------------

    profile.tenants_ownership = match ownership? {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => {
            Some(TenantsOwnership::Records(records))
        }
        _ => panic!(
            "Paginated ownership not implemented when fetch profile from email"
        ),
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ProfileResponse::RegisteredUser(profile))
}
