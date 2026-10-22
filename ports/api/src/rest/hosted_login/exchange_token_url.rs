use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{
    http::header::{HeaderValue, CACHE_CONTROL},
    post, web, HttpResponse, Responder,
};
use myc_core::{
    domain::dtos::user::User, models::AccountLifeCycle,
    use_cases::role_scoped::beginner::hosted_login::exchange_hosted_login_code,
};
use myc_http_tools::{
    functions::encode_jwt, models::internal_auth_config::InternalOauthConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shaku::HasComponent;
use tracing::warn;

/// Access token request (RFC 6749 §4.1.3, RFC 7636 §4.5).
#[derive(Deserialize)]
pub struct TokenForm {
    grant_type: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    code_verifier: Option<String>,
}

/// Successful token response (RFC 6749 §5.1) plus the Mycelium user.
#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: i64,
    user: User,
}

/// Exchange the one-time authorization code for a Mycelium session token.
#[post("/token")]
pub async fn exchange_token_url(
    form: web::Form<TokenForm>,
    sql_app_module: web::Data<SqlAppModule>,
    auth_config: web::Data<InternalOauthConfig>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
) -> impl Responder {
    let form = form.into_inner();

    if form.grant_type.as_deref() != Some("authorization_code") {
        return token_error("unsupported_grant_type");
    }

    let (Some(code), Some(redirect_uri), Some(client_id), Some(verifier)) = (
        form.code,
        form.redirect_uri,
        form.client_id,
        form.code_verifier,
    ) else {
        return token_error("invalid_request");
    };

    let user = match exchange_hosted_login_code(
        client_id,
        redirect_uri,
        code,
        verifier,
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(user) => user,
        Err(err) => {
            warn!("Hosted login code exchange refused: {}", err);
            return token_error("invalid_grant");
        }
    };

    match encode_jwt(
        user.to_owned(),
        auth_config.get_ref().to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        false,
    )
    .await
    {
        Err(response) => response,
        Ok((token, duration)) => HttpResponse::Ok()
            .insert_header((
                CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            ))
            .json(TokenResponse {
                access_token: token,
                token_type: "Bearer",
                expires_in: duration.num_seconds(),
                user,
            }),
    }
}

/// Error response (RFC 6749 §5.2).
fn token_error(error: &str) -> HttpResponse {
    HttpResponse::BadRequest()
        .insert_header((CACHE_CONTROL, HeaderValue::from_static("no-store")))
        .json(json!({ "error": error }))
}
