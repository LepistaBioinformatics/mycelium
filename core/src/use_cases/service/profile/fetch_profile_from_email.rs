use crate::{
    domain::{
        dtos::{email::Email, profile::Profile},
        entities::{ProfileFetching, TokenRegistration},
    },
    use_cases::service::token::register_token,
};

use clean_base::{
    entities::default_response::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
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
    token_registration_repo: Box<&dyn TokenRegistration>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile from database
    // ? -----------------------------------------------------------------------

    let profile = match profile_fetching_repo.get(email.to_owned()).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(_) => {
                return Ok(ProfileResponse::UnregisteredUser(email))
            }
            FetchResponseKind::Found(profile) => profile,
        },
    };

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
