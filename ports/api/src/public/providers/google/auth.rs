use crate::models::auth_config::AuthConfig;

use actix_web::web;
use myc_config::optional_config::OptionalConfig;
use reqwest::{Client, Url};
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, Debug)]
pub(crate) struct OAuthResponse {
    pub access_token: String,
    pub id_token: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GoogleUserResult {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
    pub locale: String,
}

pub(crate) async fn request_token(
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

    let redirect_url = config.oauth_redirect_url.to_owned();
    let client_secret = config.oauth_client_secret.to_owned();
    let client_id = config.oauth_client_id.to_owned();

    let root_url = "https://oauth2.googleapis.com/token";
    let client = Client::new();

    let params = [
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_url.as_str()),
        ("client_id", client_id.as_str()),
        ("code", authorization_code),
        ("client_secret", client_secret.as_str()),
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

pub(crate) async fn get_google_user(
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
