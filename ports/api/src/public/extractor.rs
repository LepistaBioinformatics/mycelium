use crate::{
    settings::{STANDARD_SERVICE_NAME, TOKENS_VALIDATION_PATH},
    utils::JsonError,
};

use actix_web::{HttpRequest, HttpResponse};
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use log::warn;
use myc_core::{
    domain::{
        dtos::{profile::Profile, token::Token},
        entities::TokenDeregistration,
    },
    settings::DEFAULT_PROFILE_KEY,
    use_cases::roles::service::profile::ProfilePack,
};
use myc_svc::repositories::TokenDeregistrationSvcRepository;

/// Extract the `Profile` from HTTP request.
///
///
/// Try to extract the profile data transfer object (`Profile`) JSON
/// representation from the Actix Web based HTTP request. The JSON extraction is
/// trying to be done from the request header. If the JSON string containing the
/// profile is not extracted, returns a `HttpResponse` with 403 status code.
pub async fn extract_profile(
    req: HttpRequest,
) -> Result<Profile, HttpResponse> {
    let pack = match try_extract_from_headers(req.to_owned()).await {
        Err(err) => {
            warn!("Unexpected error on check profile: {err}");
            return Err(HttpResponse::Forbidden().json(JsonError::new(
                "Could not check user identity.".to_string(),
            )));
        }
        Ok(res) => res,
    };

    if check_token(pack.to_owned()).await {
        return Ok(pack.profile);
    }

    Err(HttpResponse::Forbidden()
        .json(JsonError::new("Unidentified user.".to_string())))
}

async fn check_token(pack: ProfilePack) -> bool {
    let repo = TokenDeregistrationSvcRepository {
        url: TOKENS_VALIDATION_PATH.to_string(),
    };

    match repo
        .get_then_delete(Token {
            token: pack.token,
            own_service: STANDARD_SERVICE_NAME.to_string(),
        })
        .await
    {
        Err(err) => {
            warn!("Unexpected error on validate token: {err}");
            false
        }
        Ok(res) => match res {
            FetchResponseKind::NotFound(_) => false,
            FetchResponseKind::Found(_) => true,
        },
    }
}

async fn try_extract_from_headers(
    req: HttpRequest,
) -> Result<ProfilePack, MappedErrors> {
    match req.headers().get(DEFAULT_PROFILE_KEY) {
        None => Err(execution_err(
            String::from("Unable to fetch profile from header."),
            None,
            None,
        )),
        Some(res) => match res.to_str() {
            Err(err) => Err(execution_err(
                format!("Unable to fetch profile from header: {err}"),
                None,
                None,
            )),
            Ok(res) => match serde_json::from_str::<ProfilePack>(&res) {
                Err(err) => Err(execution_err(
                    format!("Unable to fetch profile from header: {err}"),
                    None,
                    None,
                )),
                Ok(res) => Ok(res),
            },
        },
    }
}
