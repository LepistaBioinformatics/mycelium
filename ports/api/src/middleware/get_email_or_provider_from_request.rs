use crate::middleware::parse_issuer_from_request;

use actix_web::{error::ParseError, web, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use awc::http::header::Header;
use jsonwebtoken::errors::ErrorKind;
use myc_config::optional_config::OptionalConfig;
use myc_http_tools::{
    functions::decode_jwt_hs512,
    models::{
        auth_config::AuthConfig,
        external_providers_config::ExternalProviderConfig,
        internal_auth_config::InternalOauthConfig,
    },
    responses::GatewayError,
    settings::MYCELIUM_PROVIDER_KEY,
    Email,
};

#[tracing::instrument(name = "get_email_or_provider_from_request", skip_all)]
pub(super) async fn get_email_or_provider_from_request(
    req: HttpRequest,
) -> Result<(Option<Email>, Option<ExternalProviderConfig>, String), GatewayError>
{
    // ? -----------------------------------------------------------------------
    // ? Extract auth config from request
    //
    // Auth config must be available in the request after injected on the API
    // initialization. If not, returns an InternalServerError.
    //
    // ? -----------------------------------------------------------------------

    let req_auth_config = if let Some(config) =
        req.app_data::<web::Data<AuthConfig>>()
    {
        config
    } else {
        tracing::error!(
            "Unable to extract AuthConfig from request. Authentication will not be completed"
        );

        return Err(GatewayError::InternalServerError(
            "Unable to initialize auth config".to_string(),
        ));
    };

    // ? -----------------------------------------------------------------------
    // ? Extract issuer from request
    //
    // Issuer should be used to start the token validation process.
    //
    // ? -----------------------------------------------------------------------

    let (issuer, token) = parse_issuer_from_request(req.clone()).await?;

    tracing::trace!("Issuer: {}", issuer);

    // ? -----------------------------------------------------------------------
    // ? Try to fetch email using internal provider
    //
    // The internal provider is used to fetch the email from the request. If
    // the email is found, returns a tuple with the email, None and the auth.
    //
    // ? -----------------------------------------------------------------------

    if issuer.to_lowercase() == MYCELIUM_PROVIDER_KEY.to_string().to_lowercase()
    {
        if let Some(email) =
            extract_email_from_internal_provider(req.clone()).await?
        {
            return Ok((Some(email), None, token));
        } else {
            return Err(GatewayError::Unauthorized(
                "Invalid issuer".to_string(),
            ));
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Try to fetch email from external providers
    //
    // The external providers are used to fetch the email from the request.
    // If the email is found, returns a tuple with the email, None and the auth.
    //
    // ? -----------------------------------------------------------------------

    let external_providers = if let OptionalConfig::Enabled(config) =
        &req_auth_config.external
    {
        config
    } else {
        tracing::error!(
            "Unable to extract external providers from request. Authentication will not be completed"
        );

        return Err(GatewayError::Unauthorized(
            "Authentication with external providers disabled".to_string(),
        ));
    };

    for provider in external_providers {
        let local_issuer =
            provider.issuer.async_get_or_error().await.map_err(|_| {
                GatewayError::Unauthorized(
                    "Could not check issuer.".to_string(),
                )
            })?;

        if local_issuer.to_lowercase() == issuer.to_lowercase() {
            return Ok((None, Some(provider.to_owned()), token));
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Return error
    //
    // If the issuer is not valid, returns an Unauthorized error.
    //
    // ? -----------------------------------------------------------------------

    tracing::error!("Invalid issuer: {}", issuer);

    Err(GatewayError::Unauthorized(
        "Token issuer not found".to_string(),
    ))
}

/// Try to fetch email from internal provider
///
/// This function is used to fetch the email from the request using the internal
/// provider. If the email is found, returns a tuple with the email, None and the
/// auth.
///
#[tracing::instrument(name = "try_to_fetch_from_internal_provider", skip_all)]
async fn extract_email_from_internal_provider(
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    tracing::trace!("Checking credentials with Mycelium Auth");
    //
    // Extract the internal OAuth2 configuration from the HTTP request. If
    // the configuration is not available returns a None.
    //
    let req_auth_config = match req.app_data::<web::Data<InternalOauthConfig>>()
    {
        Some(config) => config.jwt_secret.to_owned(),
        None => return Err(GatewayError::InternalServerError(
            "Unexpected error on validate internal auth config. Please contact the system administrator.".to_string(),
        )),
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
    let token = match Authorization::<Bearer>::parse(&req) {
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
    match decode_jwt_hs512(token, jwt_token) {
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
}
