use crate::providers::shared::check_token_online;

use super::{auth::GoogleUserResult, config::GoogleOauthConfig};

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use jsonwebtoken::{decode, DecodingKey, Validation};
use log::debug;
use myc_core::domain::dtos::email::Email;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
pub struct GoogleDecode {
    pub email: String,

    #[serde(deserialize_with = "bool_from_string")]
    pub email_verified: bool,
}

fn bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(serde::de::Error::custom("invalid value")),
    }
}

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
