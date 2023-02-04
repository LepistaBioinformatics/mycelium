use crate::{
    domain::{
        dtos::{email::Email, profile::Profile},
        entities::{
            LicensedResourcesFetching, ProfileFetching, TokenRegistration,
        },
    },
    use_cases::roles::service::{
        profile::fetch_profile_from_email::{
            fetch_profile_from_email, ProfileResponse,
        },
        token::register_token,
    },
};

use clean_base::{
    entities::default_response::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use log::debug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// DEPRECATED
///
/// The profile pack contains a token validation used during the client check of
/// the request profile. Such struct should be deprecated.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePack {
    pub profile: Profile,
    pub token: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfilePackResponse {
    RegisteredUser(ProfilePack),
    UnregisteredUser(Email),
}

/// Fetch the user profile pack from email address.
///
/// Together the profile a token is registered and their id is returned to be
/// used during the response validation. The response object containing both the
/// Profile plus the token denotes the profile pack.
pub async fn fetch_profile_pack_from_email(
    email: Email,
    requesting_service: String,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
    token_registration_repo: Box<&dyn TokenRegistration>,
) -> Result<ProfilePackResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let profile = match fetch_profile_from_email(
        email,
        profile_fetching_repo,
        licensed_resources_fetching_repo,
    )
    .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            ProfileResponse::UnregisteredUser(res) => {
                return Ok(ProfilePackResponse::UnregisteredUser(res))
            }
            ProfileResponse::RegisteredUser(res) => res,
        },
    };

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

    Ok(ProfilePackResponse::RegisteredUser(ProfilePack {
        profile,
        token: token.token,
    }))
}
