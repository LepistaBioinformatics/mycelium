use crate::{
    domain::{
        dtos::{email::Email, profile::Profile},
        entities::{
            LicensedResourcesFetching, ProfileFetching, TokenRegistration,
        },
    },
    use_cases::service::token::register_token,
};

use clean_base::{
    entities::default_response::{
        CreateResponseKind, FetchManyResponseKind, FetchResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use futures::future;
use log::debug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePack {
    pub profile: Profile,
    pub token: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    RegisteredUser(ProfilePack),
    UnregisteredUser(Email),
}

/// Fetch the user profile from email address.
///
/// Together the profile a token is registered and their id is returned to be
/// used during the response validation.
pub async fn fetch_profile_from_email(
    email: Email,
    requesting_service: String,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
    token_registration_repo: Box<&dyn TokenRegistration>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let (profile, licenses) = future::join(
        profile_fetching_repo.get(email.to_owned()),
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
    // ? Register a new token
    // ? -----------------------------------------------------------------------

    let token =
        match register_token(requesting_service, token_registration_repo).await
        {
            Err(err) => return Err(err),
            Ok(res) => match res {
                CreateResponseKind::NotCreated(_, msg) => {
                    return Err(use_case_err(msg, None, None))
                }
                CreateResponseKind::Created(token) => token,
            },
        };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ProfileResponse::RegisteredUser(ProfilePack {
        profile,
        token: token.token,
    }))
}
