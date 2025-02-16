use crate::{
    dtos::claims::Claims, models::internal_auth_config::InternalOauthConfig,
    settings::MYCELIUM_PROVIDER_KEY, utils::HttpJsonResponse,
};

use actix_web::HttpResponse;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use myc_core::{domain::dtos::user::User, models::AccountLifeCycle};
use tracing::error;

/// Encode a user into a JWT token
pub async fn encode_jwt(
    user: User,
    auth_config: InternalOauthConfig,
    core_config: AccountLifeCycle,
    is_temporary: bool,
) -> Result<(String, Duration), HttpResponse> {
    let expires_in = match match is_temporary {
        true => auth_config.tmp_expires_in,
        false => auth_config.jwt_expires_in,
    }
    .async_get_or_error()
    .await
    {
        Ok(exp) => exp,
        Err(err) => {
            error!("Could not get token expiration: {err}");

            return Err(HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message(
                    "Could not get token expiration.".to_string(),
                ),
            ));
        }
    };

    let duration = chrono::Duration::seconds(expires_in);

    let expiration = match Utc::now().checked_add_signed(duration) {
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
        iat: Utc::now().timestamp(),
        sub: match user.id.to_owned() {
            Some(id) => id.to_string(),
            None => "".to_string(),
        },
        email: user.email.email(),
        exp: expiration,
        iss: MYCELIUM_PROVIDER_KEY.to_string(),
        aud: core_config
            .domain_url
            .ok_or(core_config.domain_name)
            .map_err(|err| {
                error!("Could not get domain URL: {err:?}");

                HttpResponse::InternalServerError().json(
                    HttpJsonResponse::new_message(
                        "Unexpected error on build JWT claims".to_string(),
                    ),
                )
            })?
            .async_get_or_error()
            .await
            .map_err(|err| {
                error!("Could not get domain URL: {err:?}");

                HttpResponse::InternalServerError().json(
                    HttpJsonResponse::new_message(
                        "Unexpected error on build JWT claims".to_string(),
                    ),
                )
            })?,
    };

    let header = Header::new(Algorithm::HS512);

    let secret = match auth_config.jwt_secret.async_get_or_error().await {
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
        Ok(token) => Ok((token, duration)),
        Err(err) => Err(HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string()))),
    }
}
