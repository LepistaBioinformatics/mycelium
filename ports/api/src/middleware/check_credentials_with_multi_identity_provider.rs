use crate::middleware::parse_issuer_from_request;

use actix_web::{error::ParseError, http::header::Header, web, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use jsonwebtoken::errors::ErrorKind;
use myc_config::optional_config::OptionalConfig;
use myc_core::domain::dtos::email::Email;
use myc_http_tools::{
    functions::decode_jwt_hs512,
    models::{
        auth_config::AuthConfig, internal_auth_config::InternalOauthConfig,
    },
    providers::{az_check_credentials, gc_check_credentials},
    responses::GatewayError,
};

/// Try to populate profile to request header
///
/// This function is used to check credentials from multiple identity providers.
#[tracing::instrument(
    name = "check_credentials_with_multi_identity_provider",
    skip_all
)]
pub(crate) async fn check_credentials_with_multi_identity_provider(
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    let issuer = parse_issuer_from_request(req.clone()).await?;
    tracing::trace!("Issuer: {:?}", issuer);

    discover_provider(issuer.to_owned().to_lowercase(), req).await
}

/// Discover identity provider
///
/// This function is used to discover identity provider and check credentials.
#[tracing::instrument(name = "discover_provider", skip_all)]
async fn discover_provider(
    auth_provider: String,
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    let provider = if auth_provider.contains("sts.windows.net")
        || auth_provider.contains("azure-ad")
    {
        tracing::trace!("Checking credentials with Azure AD");
        az_check_credentials(req).await
    } else if auth_provider.contains("google") {
        tracing::trace!("Checking credentials with Google OAuth2");
        //
        // Try to extract authentication configurations from HTTP request.
        //
        let req_auth_config = req.app_data::<web::Data<AuthConfig>>();
        //
        // If Google OAuth2 config if not available the returns a Unauthorized.
        //
        if let None = req_auth_config {
            return Err(GatewayError::Unauthorized(format!(
                "Unable to extract Google auth config from request."
            )));
        }
        //
        // If Google OAuth2 config if not available the returns a Unauthorized
        // response.
        //
        let config = match req_auth_config.unwrap().google.clone() {
            OptionalConfig::Disabled => {
                tracing::warn!(
                    "Users trying to request and the Google OAuth2 is disabled."
                );

                return Err(GatewayError::Unauthorized(format!(
                    "Unable to extract auth config from request."
                )));
            }
            OptionalConfig::Enabled(config) => config,
        };
        //
        // Check if credentials are valid.
        //
        gc_check_credentials(req, config).await
    } else if auth_provider.contains("mycelium") {
        tracing::trace!("Checking credentials with Mycelium Auth");
        //
        // Extract the internal OAuth2 configuration from the HTTP request. If
        // the configuration is not available returns a InternalServerError
        // response.
        //
        let req_auth_config = match req
            .app_data::<web::Data<InternalOauthConfig>>()
        {
            Some(config) => config.jwt_secret.to_owned(),
            None => {
                return Err(GatewayError::InternalServerError(format!(
                        "Unexpected error on validate internal auth config. Please contact the system administrator."
                    )));
            }
        };
        //
        // Extract the token from the request. If the token is not available
        // returns a InternalServerError response.
        //
        let jwt_token = match req_auth_config.async_get_or_error().await {
            Ok(token) => token,
            Err(err) => {
                return Err(GatewayError::InternalServerError(format!(
                    "Unexpected error on get jwt token: {err}"
                )));
            }
        };
        //
        // Extract the bearer from the request. If the bearer is not available
        // returns a Unauthorized response.
        //
        let auth = match Authorization::<Bearer>::parse(&req) {
            Err(err) => match err {
                ParseError::Header => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Bearer token not found or invalid in request: {err}"
                    )));
                }
                _ => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Invalid Bearer token: {err}"
                    )));
                }
            },
            Ok(res) => res,
        };
        //
        // Decode the JWT token. If the token is not valid returns a
        // Unauthorized response.
        //
        match decode_jwt_hs512(auth, jwt_token) {
            Err(err) => match err.kind() {
                ErrorKind::ExpiredSignature => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Expired token: {err}"
                    )));
                }
                _ => {
                    return Err(GatewayError::Unauthorized(format!(
                        "Unexpected error on decode jwt token: {err}"
                    )))
                }
            },
            Ok(res) => {
                let claims = res.claims;
                let email = claims.email;

                match Email::from_string(email) {
                    Err(err) => {
                        return Err(GatewayError::Unauthorized(format!(
                            "Invalid email: {err}"
                        )));
                    }
                    Ok(res) => return Ok(Some(res)),
                }
            }
        }
    } else {
        return Err(GatewayError::Unauthorized(format!(
            "Unknown identity provider: {auth_provider}",
        )));
    };

    match provider {
        Err(err) => {
            let msg =
                format!("Unexpected error on match Oauth2 provider: {err}");

            tracing::warn!("Unexpected error on discovery provider: {msg}");
            Err(GatewayError::Forbidden(msg))
        }
        Ok(res) => {
            tracing::trace!("Requesting Email: {:?}", res);
            Ok(Some(res))
        }
    }
}
