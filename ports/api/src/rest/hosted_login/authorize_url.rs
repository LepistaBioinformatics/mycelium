use super::{
    hosted_login_session_cookie::hosted_login_session_cookie,
    render_hosted_login_page::{
        domain_name, render_hosted_login_page,
        render_invalid_authorization_request, render_staff_bootstrap_pending,
    },
    HOSTED_LOGIN_PATH,
};
use crate::{
    middleware::sign_in_blocked_by_staff_bootstrap,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{
    get, http::StatusCode, web, HttpRequest, HttpResponse, Responder,
};
use myc_config::optional_config::OptionalConfig;
use myc_core::{
    domain::dtos::hosted_login::{HostedLoginClients, HostedLoginRequest},
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::hosted_login::start_hosted_login,
};
use myc_http_tools::models::auth_config::AuthConfig;
use serde::Deserialize;
use shaku::HasComponent;
use tera::Context as TeraContext;
use tracing::warn;

/// OAuth 2.0 authorization request parameters (RFC 6749 §4.1.1, RFC 7636).
#[derive(Deserialize)]
pub struct AuthorizeParams {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

/// Start the hosted email login: validate the client and render the page
/// asking for the email address.
#[get("/authorize")]
pub async fn authorize_url(
    req: HttpRequest,
    query: web::Query<AuthorizeParams>,
    auth_config: web::Data<AuthConfig>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let domain_name = domain_name(&life_cycle_settings).await;

    if sign_in_blocked_by_staff_bootstrap(&req).await {
        return render_staff_bootstrap_pending(&domain_name);
    }

    let query = query.into_inner();

    let request = HostedLoginRequest {
        client_id: query.client_id.unwrap_or_default(),
        redirect_uri: query.redirect_uri.unwrap_or_default(),
        state: query.state.unwrap_or_default(),
        code_challenge: query.code_challenge.unwrap_or_default(),
    };

    let session = match start_hosted_login(
        registered_clients(&auth_config),
        query.response_type.unwrap_or_default(),
        query.code_challenge_method.unwrap_or_default(),
        request,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(session) => session,
        Err(err) => {
            warn!("Hosted login authorization refused: {}", err);
            return render_invalid_authorization_request(&domain_name);
        }
    };

    let mut context = TeraContext::new();
    context.insert("domain_name", &domain_name);
    context.insert("csrf_token", &session.csrf_token);
    context.insert("form_action", &format!("{HOSTED_LOGIN_PATH}/email"));

    let mut response = render_hosted_login_page(
        StatusCode::OK,
        "web/hosted-login-email.html",
        &context,
    );

    let cookie = hosted_login_session_cookie(
        &session.session_id,
        uses_https(&life_cycle_settings).await,
    );

    if let Err(err) = response.add_cookie(&cookie) {
        warn!("Failed to set hosted login cookie: {}", err);
        return HttpResponse::InternalServerError().finish();
    }

    response
}

fn registered_clients(auth_config: &AuthConfig) -> HostedLoginClients {
    match &auth_config.hosted_login {
        OptionalConfig::Enabled(config) => config.clients.clone(),
        OptionalConfig::Disabled => HostedLoginClients::default(),
    }
}

async fn uses_https(life_cycle_settings: &AccountLifeCycle) -> bool {
    let Some(resolver) = &life_cycle_settings.domain_url else {
        return false;
    };

    resolver
        .async_get_or_error()
        .await
        .map(|url| url.starts_with("https://"))
        .unwrap_or(false)
}
