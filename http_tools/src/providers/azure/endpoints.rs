use super::{
    functions::oauth_client,
    models::{
        AzureTokenResponse, CsrfTokenClaims, ExtraTokenFields, QueryCode,
    },
};
use crate::{models::auth_config::AuthConfig, utils::HttpJsonResponse};

use actix_web::{get, web, HttpResponse};
use chrono::Utc;
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, DecodingKey, EncodingKey, Header,
    Validation,
};
use myc_config::optional_config::OptionalConfig;
use oauth2::{
    reqwest::async_http_client, AuthorizationCode, CsrfToken,
    PkceCodeChallenge, Scope, TokenResponse,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;
use tracing::error;

pub fn configure(conf: &mut web::ServiceConfig) {
    conf.service(login).service(callback);
}

#[get("/login")]
pub async fn login(config: web::Data<AuthConfig>) -> HttpResponse {
    let config = if let OptionalConfig::Enabled(config) =
        config.get_ref().azure.to_owned()
    {
        config
    } else {
        error!("Azure OAuth is disabled");
        return HttpResponse::InternalServerError().finish();
    };

    let client = match oauth_client(config.to_owned()) {
        Ok(client) => client,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err.msg()));
        }
    };

    let csrf: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let claims = CsrfTokenClaims {
        exp: (Utc::now().timestamp() + config.csrf_token_expiration) as usize,
        csrf,
    };

    let jwt_secret = match config.jwt_secret.get_or_error() {
        Ok(secret) => secret,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
    };

    let csrf_token_jwt = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    ) {
        Ok(token) => token,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .body("Error on CSRF token generation");
        }
    };

    let (challenge, _) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(challenge)
        .url();

    HttpResponse::Ok().json(json!({
        "authorize_url": authorize_url.to_string(),
        "csrf_token": csrf_token_jwt
    }))
}

#[get("/callback")]
pub async fn callback(
    csrf_token_jwt: String,
    query: web::Query<QueryCode>,
    config: web::Data<AuthConfig>,
) -> HttpResponse {
    let config = if let OptionalConfig::Enabled(config) =
        config.get_ref().azure.to_owned()
    {
        config
    } else {
        error!("Azure OAuth is disabled");
        return HttpResponse::InternalServerError().finish();
    };

    let client = match oauth_client(config.to_owned()) {
        Ok(client) => client,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err.msg()));
        }
    };

    let jwt_secret = match config.jwt_secret.get_or_error() {
        Ok(secret) => secret,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
    };

    //
    // Decode CSRF Token
    //
    // If the token is invalid or expired, return an error
    //
    let _ = match decode::<CsrfTokenClaims>(
        &csrf_token_jwt,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(token_data) => token_data.claims,
        Err(err) => match *err.kind() {
            ErrorKind::ExpiredSignature => {
                return HttpResponse::Unauthorized().json(
                    HttpJsonResponse::new_message(
                        "CSRF Token expired".to_owned(),
                    ),
                );
            }
            _ => {
                return HttpResponse::Unauthorized().json(
                    HttpJsonResponse::new_message(
                        "Invalid CSRF Token".to_owned(),
                    ),
                )
            }
        },
    };

    let code = AuthorizationCode::new(query.code.clone());

    let token_result = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await;

    match token_result {
        Ok(token) => {
            let access_token = token.access_token();

            let token_response = AzureTokenResponse::new(
                access_token.to_owned(),
                token.token_type().to_owned(),
                ExtraTokenFields {},
            );

            HttpResponse::Ok().json(token_response)
        }
        Err(_) => {
            HttpResponse::InternalServerError().body("Error on token exchange")
        }
    }
}
