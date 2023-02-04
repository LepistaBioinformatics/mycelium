use crate::{domain::dtos::token::Claims, settings::BEARER_TOKEN_SECRET};

use chrono::{Duration, Utc};
use clean_base::utils::errors::{use_case_err, MappedErrors};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

/// Generate a Bearer token including user email
///
///
/// WARNING
/// -------
///
/// Token generated here should only be used in development or test
/// environments.
///
pub async fn generate_bearer_token_from_email(
    email: String,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Collect Bearer token
    // ? -----------------------------------------------------------------------

    let tmp_secret = &*BEARER_TOKEN_SECRET.lock().await;

    let secret = match tmp_secret {
        None => panic!("Bearer secret not configured."),
        Some(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Encode token
    // ? -----------------------------------------------------------------------

    let expiration = Utc::now()
        .checked_add_signed(Duration::days(60))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        email,
        exp: expiration as usize,
    };

    let header = Header::new(Algorithm::default());

    match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ) {
        Err(err) => Err(use_case_err(format!("{err}"), None, None)),
        Ok(res) => Ok(res),
    }
}
