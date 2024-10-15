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
) -> Result<String, HttpResponse> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(token.jwt_expires_in))
        .expect("valid timestamp")
        .timestamp();

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
                HttpJsonResponse::new_message("Could not get token secret key.".to_string()),
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
