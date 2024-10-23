use crate::dtos::MyceliumProfileData;

use actix_web::{error::ParseError, http::header::Header, web, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use jsonwebtoken::errors::ErrorKind;
use jwt::{Header as JwtHeader, RegisteredClaims, Token};
use myc_config::optional_config::OptionalConfig;
use myc_core::{
    domain::dtos::email::Email,
    use_cases::roles::service::profile::{
        fetch_profile_from_email, ProfileResponse,
    },
};
use myc_http_tools::{
    functions::decode_jwt_hs512,
    models::{
        auth_config::AuthConfig, internal_auth_config::InternalOauthConfig,
    },
    providers::{az_check_credentials, gc_check_credentials},
    responses::GatewayError,
};
use myc_prisma::repositories::{
    LicensedResourcesFetchingSqlDbRepository, ProfileFetchingSqlDbRepository,
};
use tracing::{debug, warn};

/// Try to populate profile to request header
///
/// This function is auxiliary of the MyceliumProfileData struct used to extract
/// the Mycelium Profile from the request on mycelium native APIs.
#[tracing::instrument(name = "fetch_profile_from_request", skip_all)]
pub(crate) async fn fetch_profile_from_request(
    req: HttpRequest,
) -> Result<MyceliumProfileData, GatewayError> {
    let email =
        check_credentials_with_multi_identity_provider(req.clone()).await?;

    if email.is_none() {
        return Err(GatewayError::Unauthorized(format!(
            "Unable o extract user identity from request."
        )));
    }

    let profile = match fetch_profile_from_email(
        email.to_owned().unwrap(),
        Box::new(&ProfileFetchingSqlDbRepository {}),
        Box::new(&LicensedResourcesFetchingSqlDbRepository {}),
    )
    .await
    {
        Err(err) => {
            let msg =
                format!("Unexpected error on fetch profile from email: {err}");

            warn!("{msg}");
            return Err(GatewayError::InternalServerError(msg));
        }
        Ok(res) => match res {
            ProfileResponse::UnregisteredUser(email) => {
                return Err(GatewayError::Forbidden(format!(
                    "Unauthorized access: {:?}",
                    email,
                )))
            }
            ProfileResponse::RegisteredUser(res) => res,
        },
    };

    Ok(MyceliumProfileData::from_profile(profile))
}

/// Try to populate profile to request header
///
/// This function is used to check credentials from multiple identity providers.
#[tracing::instrument(
    name = "check_credentials_with_multi_identity_provider",
    skip_all
)]
pub async fn check_credentials_with_multi_identity_provider(
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    let issuer = parse_issuer_from_request(req.clone()).await?;
    discover_provider(issuer.to_owned().to_lowercase(), req).await
}

/// Parse issuer from request
///
/// This function is used to parse issuer from request.
#[tracing::instrument(name = "parse_issuer_from_request", skip_all)]
pub async fn parse_issuer_from_request(
    req: HttpRequest,
) -> Result<String, GatewayError> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            return Err(GatewayError::Unauthorized(format!(
                "Unexpected error on get bearer from request: {err}"
            )));
        }
        Ok(res) => res,
    }
    .to_string()
    .replace("Bearer ", "")
    .replace("bearer ", "");

    let unverified: Token<JwtHeader, RegisteredClaims, _> =
        match Token::parse_unverified(&auth) {
            Err(err) => {
                let msg = format!(
                    "Unexpected error on parse unverified token: {err}"
                );

                warn!("{msg}");
                return Err(GatewayError::Unauthorized(msg));
            }
            Ok(res) => res,
        };

    let issuer = unverified.claims().issuer.as_ref().ok_or(
        GatewayError::Unauthorized("Could not check issuer.".to_string()),
    )?;

    Ok(issuer.to_owned().to_lowercase())
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
        az_check_credentials(req).await
    } else if auth_provider.contains("google") {
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
                warn!(
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
        let jwt_token = match req_auth_config.get_or_error() {
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

            warn!("Unexpected error on discovery provider: {msg}");
            Err(GatewayError::Forbidden(msg))
        }
        Ok(res) => {
            debug!("Requesting Email: {:?}", res);
            Ok(Some(res))
        }
    }
}
