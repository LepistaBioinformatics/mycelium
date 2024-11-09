use crate::{
    dtos::claims::Claims, models::internal_auth_config::InternalOauthConfig,
    utils::HttpJsonResponse,
};

use actix_web::HttpResponse;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use myc_core::domain::dtos::user::User;

/// Encode a user into a JWT token
pub fn encode_jwt(
    user: User,
    token: InternalOauthConfig,
    is_temporary: bool,
) -> Result<String, HttpResponse> {
    let expires_in = match is_temporary {
        true => token.tmp_expires_in,
        false => token.jwt_expires_in,
    };

    let expiration = match Utc::now()
        .checked_add_signed(chrono::Duration::seconds(expires_in))
    {
        Some(exp) => exp.timestamp(),
        None => {
            return Err(HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message(
                    "Could not calculate token expiration.".to_string(),
                ),
            ));
        }
    };

    let claims = Claims {
        sub: match user.id.to_owned() {
            Some(id) => id.to_string(),
            None => "".to_string(),
        },
        email: user.email.get_email(),
        exp: expiration,
        iss: "mycelium".to_string(),
    };

    let header = Header::new(Algorithm::HS512);

    let secret = match token.jwt_secret.get_or_error() {
        Ok(key) => key,
        Err(_) => {
            return Err(HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message(
                    "Could not get token secret key.".to_string(),
                ),
            ));
        }
    };

    match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ) {
        Ok(token) => Ok(token),
        Err(err) => Err(HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string()))),
    }
}
