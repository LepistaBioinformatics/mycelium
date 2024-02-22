use crate::dtos::MyceliumProfileData;

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use jwt::{Header as JwtHeader, RegisteredClaims, Token};
use log::{debug, warn};
use myc_config::optional_config::OptionalConfig;
use myc_core::{
    domain::dtos::email::Email,
    use_cases::roles::service::profile::{
        fetch_profile_from_email, ProfileResponse,
    },
};
use myc_http_tools::{
    models::auth_config::AuthConfig,
    providers::{az_check_credentials, gc_check_credentials},
    responses::GatewayError,
};
use myc_prisma::repositories::{
    LicensedResourcesFetchingSqlDbRepository, ProfileFetchingSqlDbRepository,
};

/// Try to populate profile to request header
pub(crate) async fn fetch_profile_from_request(
    req: HttpRequest,
) -> Result<MyceliumProfileData, GatewayError> {
    let email =
        check_credentials_with_multi_identity_provider(req.clone()).await?;

    if email.is_none() {
        return Err(GatewayError::Forbidden(format!(
            "Unable o extract user identity from request."
        )));
    }

    let profile_fetching_repo = req
        .app_data::<ProfileFetchingSqlDbRepository>()
        .ok_or(GatewayError::InternalServerError(
            "Unable to extract profile repository from request.".to_string(),
        ))?;

    let licensed_resources_fetching_repo = req
        .app_data::<LicensedResourcesFetchingSqlDbRepository>()
        .ok_or(GatewayError::InternalServerError(
            "Unable to extract licensed resources repository from request."
                .to_string(),
        ))?;

    let profile = match fetch_profile_from_email(
        email.to_owned().unwrap(),
        Box::new(profile_fetching_repo),
        Box::new(licensed_resources_fetching_repo),
    )
    .await
    {
        Err(err) => {
            warn!("{:?}", err);
            return Err(GatewayError::InternalServerError(format!("{err}")));
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
pub async fn check_credentials_with_multi_identity_provider(
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    let issuer = parse_issuer_from_request(req.clone()).await?;
    discover_provider(issuer.to_owned().to_lowercase(), req).await
}

/// Parse issuer from request
///
/// This function is used to parse issuer from request.
pub async fn parse_issuer_from_request(
    req: HttpRequest,
) -> Result<String, GatewayError> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            return Err(GatewayError::Forbidden(format!("{err}")));
        }
        Ok(res) => res,
    }
    .to_string();

    let token = auth.replace("Bearer ", "");

    let unverified: Token<JwtHeader, RegisteredClaims, _> =
        match Token::parse_unverified(&token) {
            Err(err) => {
                warn!("{:?}", err);
                return Err(GatewayError::Forbidden(format!("{err}")));
            }
            Ok(res) => res,
        };

    let issuer =
        unverified
            .claims()
            .issuer
            .as_ref()
            .ok_or(GatewayError::Forbidden(
                "Could not check issuer.".to_string(),
            ))?;

    Ok(issuer.to_owned().to_lowercase())
}

/// Discover identity provider
///
/// This function is used to discover identity provider and check credentials.
async fn discover_provider(
    auth_provider: String,
    req: HttpRequest,
) -> Result<Option<Email>, GatewayError> {
    let provider = if auth_provider.contains("sts.windows.net") ||
        auth_provider.contains("azure-ad")
    {
        az_check_credentials(req).await
    } else if auth_provider.contains("google") {
        //
        // Try to extract authentication configurations from HTTP request.
        //
        let req_auth_config = req.app_data::<AuthConfig>().clone();
        //
        // If Google OAuth2 config if not available the returns a Forbidden.
        //
        if let None = req_auth_config {
            return Err(GatewayError::Forbidden(format!(
                "Unable to extract Google auth config from request."
            )));
        }
        //
        // If Google OAuth2 config if not available the returns a Forbidden
        // response.
        //
        let config = match req_auth_config.unwrap().google.clone() {
            OptionalConfig::Disabled => {
                warn!(
                    "Users trying to request and the Google OAuth2 is disabled."
                );

                return Err(GatewayError::Forbidden(format!(
                    "Unable to extract auth config from request."
                )));
            }
            OptionalConfig::Enabled(config) => config,
        };
        //
        // Check if credentials are valid.
        //
        gc_check_credentials(req, config).await
    } else {
        return Err(GatewayError::Forbidden(format!(
            "Unknown identity provider: {auth_provider}",
        )));
    };

    match provider {
        Err(err) => {
            let msg =
                format!("Unexpected error on match Oauth2 provider: {err}");

            warn!("{msg}");
            Err(GatewayError::Forbidden(msg))
        }
        Ok(res) => {
            debug!("Requesting Email: {:?}", res);
            Ok(Some(res))
        }
    }
}
