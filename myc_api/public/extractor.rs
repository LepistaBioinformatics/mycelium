use actix_web::{HttpRequest, HttpResponse};
use clean_base::utils::errors::{execution_err, MappedErrors};
use myc_core::{
    domain::dtos::profile::ProfileDTO, settings::DEFAULT_PROFILE_KEY,
};

/// Extract the `ProfileDTO` from HTTP request.
///
///
/// Try to extract the profile data transfer object (`ProfileDTO`) JSON
/// representation from the Actix Web based HTTP request. The JSON extraction is
/// trying to be done from the request header and cookie. If the JSON string
/// containing the profile is not extracted from almost one of this, returns a
/// `HttpResponse` with 403 status code.
pub async fn extract_profile(
    req: HttpRequest,
) -> Result<ProfileDTO, HttpResponse> {
    match try_extract_from_headers(req.to_owned()).await {
        Err(_) => (),
        Ok(res) => return Ok(res),
    };

    match try_extract_from_cookies(req.to_owned()).await {
        Err(_) => (),
        Ok(res) => return Ok(res),
    };

    Err(HttpResponse::Forbidden().body("Unidentified user."))
}

async fn try_extract_from_headers(
    req: HttpRequest,
) -> Result<ProfileDTO, MappedErrors> {
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
            Ok(res) => {
                println!("res: {:?}", res);

                match serde_json::from_str::<ProfileDTO>(&res) {
                    Err(err) => Err(execution_err(
                        format!("Unable to fetch profile from header: {err}"),
                        None,
                        None,
                    )),
                    Ok(res) => Ok(res),
                }
            }
        },
    }
}

async fn try_extract_from_cookies(
    req: HttpRequest,
) -> Result<ProfileDTO, MappedErrors> {
    match req.cookie(DEFAULT_PROFILE_KEY) {
        None => Err(execution_err(
            String::from("Unable to fetch profile from header."),
            None,
            None,
        )),
        Some(res) => {
            match serde_json::from_str::<ProfileDTO>(&res.to_string().as_str())
            {
                Err(err) => Err(execution_err(
                    format!("Unable to fetch profile from header: {err}"),
                    None,
                    None,
                )),
                Ok(res) => Ok(res),
            }
        }
    }
}
