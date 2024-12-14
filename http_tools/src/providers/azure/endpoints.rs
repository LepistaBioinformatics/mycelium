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
    PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;
use utoipa::{IntoParams, ToResponse, ToSchema};

pub fn configure(conf: &mut web::ServiceConfig) {
    conf.service(login_url).service(token_url);
}

#[derive(Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AzureLoginResponse {
    authorize_url: String,
}

#[derive(Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CallbackResponse {
    access_token: String,
    token_type: String,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct LoginParams {
    json_return: Option<bool>,
}

/// Generate the Azure OAuth authorize URL
///
/// Users should access this URL to start the OAuth2 flow.
///
#[utoipa::path(
    get,
    params(LoginParams),
    responses(
        (
            status = 500,
            description = "Azure OAuth is disabled.",
            body = HttpJsonResponse,
        ),
        (
            status = 200,
            description = "Returns the Azure OAuth authorize URL.",
            body = AzureLoginResponse,
        )
    ),
    security(())
)]
#[get("/login")]
pub async fn login_url(
    auth_config: web::Data<AuthConfig>,
    params: web::Query<LoginParams>,
) -> HttpResponse {
    let config = if let OptionalConfig::Enabled(config) =
        auth_config.get_ref().azure.to_owned()
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

    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

    let claims = CsrfTokenClaims {
        exp: (Utc::now().timestamp() + config.csrf_token_expiration) as usize,
        csrf: csrf.to_owned(),
        code_verifier: verifier.secret().to_owned(),
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
            return HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message("Error on CSRF token generation"),
            );
        }
    };

    let state_fn = move || CsrfToken::new(csrf_token_jwt.clone());

    let (authorize_url, _) = client
        .authorize_url(state_fn)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(challenge)
        .url();

    // Return the authorize URL
    //
    // If the `json_return` parameter is set, return the URL as JSON, otherwise
    // redirect the user to the URL.
    //
    if let Some(true) = params.json_return {
        return HttpResponse::Ok().json(AzureLoginResponse {
            authorize_url: authorize_url.to_string(),
        });
    } else {
        HttpResponse::TemporaryRedirect()
            .append_header(("Location", authorize_url.to_string()))
            .finish()
    }
}

/// Callback URL for Azure OAuth
///
/// This endpoint is called by Azure after the user authorizes the application.
///
#[utoipa::path(
    get,
    responses(
        (
            status = 500,
            description = "Azure OAuth is disabled.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Code not found.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Invalid CSRF Token.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "CSRF Token expired.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Error on token exchange.",
            body = HttpJsonResponse,
        ),
        (
            status = 200,
            description = "Returns the Azure OAuth authorize URL.",
            body = CallbackResponse,
        )
    ),
    security(())
)]
#[get("/token")]
pub async fn token_url(
    query: web::Query<QueryCode>,
    auth_config: web::Data<AuthConfig>,
) -> HttpResponse {
    if let Some(err) = query.error.to_owned() {
        let error_description =
            query.error_description.to_owned().unwrap_or("".to_owned());

        error!("Error on callback: {err}: {error_description}");

        return HttpResponse::BadRequest().json(json!({
            "error": err,
            "description": error_description,
            "step": "callback"
        }));
    }

    let code = match query.code.to_owned() {
        Some(code) => code,
        None => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("Code not found".to_owned()),
            );
        }
    };

    let config = if let OptionalConfig::Enabled(config) =
        auth_config.get_ref().azure.to_owned()
    {
        config
    } else {
        error!("Azure OAuth is disabled");
        return HttpResponse::InternalServerError().finish();
    };

    let client = match oauth_client(config.to_owned()) {
        Ok(client) => client,
        Err(err) => {
            error!("Error on OAuth client: {err}");

            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err.msg()));
        }
    };

    let jwt_secret = match config.jwt_secret.get_or_error() {
        Ok(secret) => secret,
        Err(err) => {
            error!("Error on JWT secret: {err}");

            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
    };

    //
    // Decode CSRF Token
    //
    // If the token is invalid or expired, return an error
    //
    let csrf_claims = match decode::<CsrfTokenClaims>(
        &query.state.to_owned(),
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
                return HttpResponse::Unauthorized().json(json!({
                    "error": "Invalid CSRF Token",
                    "description": err.to_string(),
                    "step": "csrf"
                }));
            }
        },
    };

    let code = AuthorizationCode::new(code.clone());

    match client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(csrf_claims.code_verifier))
        .request_async(async_http_client)
        .await
    {
        Ok(token) => {
            let access_token = token.access_token();

            let token_response = AzureTokenResponse::new(
                access_token.to_owned(),
                token.token_type().to_owned(),
                ExtraTokenFields {},
            );

            HttpResponse::Ok().json(token_response)
        }
        Err(err) => {
            match err {
                oauth2::RequestTokenError::ServerResponse(response) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": response.error().to_string(),
                        "error_description": response.error_description(),
                        "step": "token_exchange_server"
                    }))
                }
                oauth2::RequestTokenError::Request(ref err) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": err.to_string(),
                        "step": "token_exchange_request"
                    }))
                }
                ref response => {
                    error!(
                        "Error on token exchange: {:?}",
                        response.to_string()
                    );
                }
            }

            HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message(format!(
                    "Error on token exchange: {err}"
                )),
            )
        }
    }
}
