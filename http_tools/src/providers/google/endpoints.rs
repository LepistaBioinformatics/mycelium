use super::{
    functions::{get_google_user, request_token},
    models::{QueryCode, TokenClaims},
};
use crate::models::auth_config::AuthConfig;

use actix_web::{
    cookie::{time::Duration as ActixWebDuration, Cookie},
    get, web, HttpResponse, Responder,
};
use chrono::{prelude::*, Duration};
use jsonwebtoken::{encode, EncodingKey, Header};
use log::debug;
use myc_config::optional_config::OptionalConfig;
use reqwest::header::LOCATION;

pub fn configure(conf: &mut web::ServiceConfig) {
    conf.service(google_callback_url);
}

/// Callback URL for Google Oauth2
///
/// This endpoint is called by Google after the user authorizes the application.
///
#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "Redirect user to auth url",
        )
    ),
    security(())
)]
#[get("/callback")]
pub async fn google_callback_url(
    query: web::Query<QueryCode>,
    data: web::Data<AuthConfig>,
) -> impl Responder {
    let code = &query.code;
    let state = &query.state;

    if code.is_empty() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "fail",
            "message": "Authorization code not provided!"
        }));
    }

    let token_response = match request_token(code.as_str(), &data).await {
        Err(err) => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": err.to_string()
            }));
        }
        Ok(res) => res,
    };

    let google_user = match get_google_user(
        &token_response.access_token,
        &token_response.id_token,
    )
    .await
    {
        Err(err) => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": err.to_string(),
            }));
        }
        Ok(res) => res,
    };

    let config = match data.as_ref().google.to_owned() {
        OptionalConfig::Disabled => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": "Google Oauth2 is disabled!"
            }));
        }
        OptionalConfig::Enabled(config) => config,
    };

    let jwt_max_age = match config.jwt_max_age.async_get_or_error().await {
        Ok(age) => age,
        Err(err) => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": err.to_string()
            }));
        }
    };

    let jwt_secret = match config.jwt_secret.async_get_or_error().await {
        Ok(secret) => secret,
        Err(err) => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": err.to_string()
            }));
        }
    };

    let client_origin = match config.client_origin.async_get_or_error().await {
        Ok(origin) => origin,
        Err(err) => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": err.to_string()
            }));
        }
    };

    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::minutes(jwt_max_age)).timestamp() as usize;

    let claims = TokenClaims {
        sub: google_user.id.to_owned(),
        exp,
        iat,
        iss: "https://accounts.google.com".to_string(),
        id: google_user.id.to_owned(),
        email: google_user.email.to_owned(),
        verified_email: google_user.verified_email.to_owned(),
        name: google_user.name.to_owned(),
        given_name: google_user.given_name.to_owned(),
        family_name: google_user.family_name.to_owned(),
        picture: google_user.picture.to_owned(),
        locale: google_user.locale.to_owned(),
    };

    let token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    ) {
        Ok(token) => token,
        Err(err) => {
            debug!("Error encoding token: {:?}", err);
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": "Error encoding token"
            }));
        }
    };

    HttpResponse::Ok()
        .append_header((
            LOCATION,
            format!("{}{}", client_origin.to_owned(), state),
        ))
        .cookie(
            Cookie::build("token", token.to_owned())
                .path("/")
                .max_age(ActixWebDuration::new(60 * jwt_max_age, 0))
                .http_only(true)
                .finish(),
        )
        .json(serde_json::json!({
            "status": "success",
            "token": token
        }))
}
