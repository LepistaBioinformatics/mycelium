use super::check_credentials_with_multi_identity_provider;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_EMAIL_KEY};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::warn;

/// Fetch and inject email to forward
///
/// This function is used to fetch the user email from the request and inject it
/// into the request headers.
///
#[tracing::instrument(name = "fetch_and_inject_email_to_forward", skip_all)]
pub async fn fetch_and_inject_email_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
) -> Result<ClientRequest, GatewayError> {
    let email =
        check_credentials_with_multi_identity_provider(req.clone()).await?;

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_EMAIL_KEY).unwrap(),
        match HeaderValue::from_str(
            &serde_json::to_string(&email.email()).unwrap(),
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
