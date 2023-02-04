use crate::{domain::dtos::token::Claims, settings::BEARER_TOKEN_SECRET};

use clean_base::utils::errors::{use_case_err, MappedErrors};
use jsonwebtoken::{decode, DecodingKey, Validation};

/// Decode a single Bearer token
///
/// Decode a token performing default bearer validation
/// (`jsonwebtoken::Validation::default()`).
pub(crate) async fn decode_bearer_token(
    token: String,
) -> Result<Claims, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Collect Bearer token
    // ? -----------------------------------------------------------------------

    let tmp_secret = &*BEARER_TOKEN_SECRET.lock().await;

    let secret = match tmp_secret {
        None => panic!("Bearer secret not configured."),
        Some(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Decode token
    // ? -----------------------------------------------------------------------

    match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Err(err) => Err(use_case_err(
            format!("Could not decode token: {err}"),
            None,
            None,
        )),
        Ok(res) => Ok(res.claims),
    }
}
