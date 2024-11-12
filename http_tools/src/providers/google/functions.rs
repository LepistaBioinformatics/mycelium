use super::{
    config::GoogleOauthConfig,
    models::{GoogleDecode, GoogleUserResult, OAuthResponse},
};
use crate::{
    models::auth_config::AuthConfig, providers::shared::check_token_online,
};

use actix_web::web;
use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use jsonwebtoken::{decode, DecodingKey, Validation};
use log::debug;
use myc_config::optional_config::OptionalConfig;
use myc_core::domain::dtos::email::Email;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::{Client, Url};
use std::error::Error;

/// Try to collect the user email.
///
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
pub async fn check_credentials(
    req: HttpRequest,
    config: GoogleOauthConfig,
) -> Result<Email, MappedErrors> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            return execution_err(format!("Invalid client request: {err}"))
                .as_error();
        }
        Ok(res) => res,
    };

    decode_bearer_token_on_google(auth.to_owned(), config).await
}

/// Decode the bearer token on Google.
///
/// This function is used to decode the bearer token on Google.
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
///
async fn decode_bearer_token_on_google(
    auth: Authorization<Bearer>,
    config: GoogleOauthConfig,
) -> Result<Email, MappedErrors> {
    let token = auth
        .to_string()
        .replace("Bearer ", "")
        .replace("bearer ", "");

    match decode::<GoogleUserResult>(
        &token,
        &DecodingKey::from_secret(config.jwt_secret.get_or_error()?.as_ref()),
        &Validation::default(),
    ) {
        Ok(token) => return Email::from_string(token.claims.email),
        Err(err) => {
            debug!("Error decoding token with jwt: {err}");
        }
    };

    match check_token_online::<GoogleDecode, _>(
        token,
        "https://oauth2.googleapis.com/tokeninfo",
        Some(true),
    )
    .await
    {
        Ok(token) => {
            if token.email_verified {
                return Email::from_string(token.email);
            }

            return execution_err(
                "Invalid user or user not verified.".to_string(),
            )
            .as_error();
        }
        Err(err) => {
            debug!("Error decoding token with online check: {err}");
        }
    };

    return execution_err("Error decoding Google Oauth2 token".to_string())
        .as_error();
}

pub(super) async fn request_token(
    authorization_code: &str,
    data: &web::Data<AuthConfig>,
) -> Result<OAuthResponse, Box<dyn Error>> {
    let config = match data.as_ref().google.to_owned() {
        OptionalConfig::Disabled => {
            return Err(From::from(
                "Google Oauth2 is not enabled on this server.",
            ));
        }
        OptionalConfig::Enabled(config) => config,
    };

    let redirect_url = config.redirect_url.to_owned();
    let root_url = "https://oauth2.googleapis.com/token";
    let client = Client::new();

    let client_id = match config.client_id.get_or_error() {
        Ok(res) => res,
        Err(err) => {
            return Err(From::from(format!(
                "Could not retrieve client ID: {err}"
            )));
        }
    };

    let client_secret = match config.client_secret.get_or_error() {
        Ok(res) => res,
        Err(err) => {
            return Err(From::from(format!(
                "Could not retrieve client secret: {err}"
            )));
        }
    };

    let params = [
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_url.as_str()),
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_ref()),
        ("code", authorization_code),
    ];

    let response = client.post(root_url).form(&params).send().await?;

    if response.status().is_success() {
        let oauth_response = response.json::<OAuthResponse>().await?;
        Ok(oauth_response)
    } else {
        Err(From::from(format!(
            "An error occurred while trying to retrieve access token (status {}): {}",
            response.status(),
            response.text().await?
        )))
    }
}

pub(super) async fn get_google_user(
    access_token: &str,
    id_token: &str,
) -> Result<GoogleUserResult, Box<dyn Error>> {
    let mut url = Url::parse("https://www.googleapis.com/oauth2/v1/userinfo")?;

    url.query_pairs_mut()
        .append_pair("alt", "json")
        .append_pair("access_token", access_token);

    let response = Client::new().get(url).bearer_auth(id_token).send().await?;

    if response.status().is_success() {
        return Ok(response.json::<GoogleUserResult>().await?);
    }

    Err(From::from(
        "An error occurred while trying to retrieve user information.",
    ))
}
