use crate::{responses::GatewayError, DEFAULT_PROFILE_KEY};

use actix_web::HttpRequest;
use awc::{error::HeaderValue, ClientRequest};
use log::warn;
use reqwest::header::HeaderName;
use std::str::FromStr;

use super::fetch_profile_from_request;

/// Fetch profile from email and inject on client request
///
/// Try to extract profile from email (these extracted from the bearer token)
/// and, then find the profile from the email and inject profile into the
/// forward request.
///
/// These use-case is usual over middleware or routers parts of the application.
pub async fn fetch_and_inject_profile_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
) -> Result<ClientRequest, GatewayError> {
    let profile = match fetch_profile_from_request(req).await {
        Err(err) => return Err(err),
        Ok(res) => res,
    };

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_PROFILE_KEY).unwrap(),
        match HeaderValue::from_str(
            &serde_json::to_string(&profile.to_profile()).unwrap(),
        ) {
            Err(err) => {
                warn!("err: {:?}", err.to_string());
                return Err(GatewayError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(res) => res,
        },
    );

    Ok(forwarded_req)
}
