use crate::{
    models::auth_config::AuthConfig,
    providers::{az_check_credentials, gc_check_credentials},
    responses::GatewayError,
    LicensedResources, Profile,
};

use actix_web::{dev::Payload, http::header::Header, FromRequest, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use futures::Future;
use jwt::{Header as JwtHeader, RegisteredClaims, Token};
use log::{debug, warn};
use myc_config::optional_config::OptionalConfig;
use myc_core::{
    domain::dtos::{account::VerboseStatus, email::Email, profile::Owner},
    use_cases::roles::service::profile::{
        fetch_profile_from_email, ProfileResponse,
    },
};
use myc_prisma::repositories::{
    LicensedResourcesFetchingSqlDbRepository, ProfileFetchingSqlDbRepository,
};
use serde::Deserialize;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyceliumProfileData {
    pub owner_credentials: Vec<Owner>,
    pub current_account_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
    pub owner_is_active: bool,
    pub account_is_active: bool,
    pub account_was_approved: bool,
    pub account_was_archived: bool,
    pub verbose_status: Option<VerboseStatus>,
    pub licensed_resources: Option<Vec<LicensedResources>>,
}

impl MyceliumProfileData {
    pub fn from_profile(profile: Profile) -> Self {
        Self {
            owner_credentials: profile.owner_credentials,
            current_account_id: profile.current_account_id,
            is_subscription: profile.is_subscription,
            is_manager: profile.is_manager,
            is_staff: profile.is_staff,
            owner_is_active: profile.owner_is_active,
            account_is_active: profile.account_is_active,
            account_was_approved: profile.account_was_approved,
            account_was_archived: profile.account_was_archived,
            verbose_status: profile.verbose_status,
            licensed_resources: profile.licensed_resources,
        }
    }

    pub fn to_profile(&self) -> Profile {
        Profile {
            owner_credentials: self.owner_credentials.to_owned(),
            current_account_id: self.current_account_id,
            is_subscription: self.is_subscription,
            is_manager: self.is_manager,
            is_staff: self.is_staff,
            owner_is_active: self.owner_is_active,
            account_is_active: self.account_is_active,
            account_was_approved: self.account_was_approved,
            account_was_archived: self.account_was_archived,
            verbose_status: self.verbose_status.to_owned(),
            licensed_resources: self.licensed_resources.to_owned(),
        }
    }
}

impl FromRequest for MyceliumProfileData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        Box::pin(async move { fetch_profile_from_request(req_clone).await })
    }
}

/// Try to populate profile to request header
pub(super) async fn fetch_profile_from_request(
    req: HttpRequest,
) -> Result<MyceliumProfileData, GatewayError> {
    let email = check_credentials_with_multi_identity_provider(req).await?;

    if email.is_none() {
        return Err(GatewayError::Forbidden(format!(
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
            warn!("{:?}", err);
            return Err(GatewayError::InternalServerError(format!("{err}")));
        }
        Ok(res) => {
            debug!("Requesting Profile: {:?}", res);

            match res {
                ProfileResponse::UnregisteredUser(email) => {
                    return Err(GatewayError::Forbidden(format!(
                        "Unauthorized access: {:?}",
                        email,
                    )))
                }
                ProfileResponse::RegisteredUser(res) => res,
            }
        }
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
