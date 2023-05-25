use super::{auth::GoogleUserResult, config::Config};

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use clean_base::utils::errors::{factories::execution_err, MappedErrors};
use jsonwebtoken::{decode, DecodingKey, Validation};
use log::warn;
use myc_core::domain::dtos::email::Email;

/// Try to collect the user email.
///
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
pub async fn check_credentials(
    req: HttpRequest,
) -> Result<Email, MappedErrors> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            return execution_err(format!("Invalid client request: {err}"))
                .as_error();
        }
        Ok(res) => res,
    };

    decode_bearer_token_on_google(auth.to_owned()).await
}

/// Decode the bearer token on Google.
///
/// This function is used to decode the bearer token on Google.
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
///
async fn decode_bearer_token_on_google(
    auth: Authorization<Bearer>,
) -> Result<Email, MappedErrors> {
    let config = Config::init();
    match decode::<GoogleUserResult>(
        &auth
            .to_string()
            .replace("Bearer ", "")
            .replace("bearer ", ""),
        &DecodingKey::from_secret(config.jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(token) => Email::from_string(token.claims.email),
        Err(err) => {
            warn!("Error decoding token: {:?}", err);
            return execution_err(
                "Error decoding Google Oauth2 token".to_string(),
            )
            .as_error();
        }
    }
}
