use crate::utils::JsonError;

use actix_web::{HttpRequest, HttpResponse};
use clean_base::utils::errors::{execution_err, MappedErrors};
use log::warn;
use myc_core::{domain::dtos::profile::Profile, settings::DEFAULT_PROFILE_KEY};

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
    match try_extract_from_headers(req.to_owned()).await {
        Err(err) => {
            warn!("Unexpected error on check profile: {err}");
            return Err(HttpResponse::Forbidden().json(JsonError::new(
                "Could not check user identity.".to_string(),
            )));
        }
        Ok(res) => return Ok(res),
    };
}

/// Try to extract profile from request header
async fn try_extract_from_headers(
    req: HttpRequest,
) -> Result<Profile, MappedErrors> {
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
            Ok(res) => match serde_json::from_str::<Profile>(&res) {
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
