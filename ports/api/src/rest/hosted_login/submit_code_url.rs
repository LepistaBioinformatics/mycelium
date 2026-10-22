use super::{
    build_client_redirect::build_client_redirect,
    hosted_login_session_cookie::{
        hosted_login_session_cookie, read_hosted_login_session_id,
    },
    render_hosted_login_page::{
        domain_name, render_hosted_login_error, render_hosted_login_page,
    },
    HOSTED_LOGIN_PATH,
};
use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{
    http::{header::LOCATION, StatusCode},
    post, web, HttpRequest, HttpResponse, Responder,
};
use myc_core::{
    domain::dtos::hosted_login::{HostedLoginCodeCheck, HostedLoginSession},
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::hosted_login::verify_hosted_login_code,
};
use serde::Deserialize;
use shaku::HasComponent;
use tera::Context as TeraContext;
use tracing::warn;

#[derive(Deserialize)]
pub struct CodeForm {
    csrf_token: String,
    code: String,
}

/// Check the emailed code. On success send the browser back to the client
/// with a one-time authorization code.
#[post("/code")]
pub async fn submit_code_url(
    req: HttpRequest,
    form: web::Form<CodeForm>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let domain_name = domain_name(&life_cycle_settings).await;
    let form = form.into_inner();

    let Some(session_id) = read_hosted_login_session_id(&req) else {
        return render_hosted_login_error(&domain_name);
    };

    match verify_hosted_login_code(
        session_id,
        form.csrf_token,
        form.code.trim().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(HostedLoginCodeCheck::Verified(session)) => {
            redirect_to_client(&session, &domain_name)
        }
        Ok(HostedLoginCodeCheck::Rejected(session)) => {
            render_code_page_with_error(&session, &domain_name)
        }
        Ok(HostedLoginCodeCheck::NotFound) => {
            render_hosted_login_error(&domain_name)
        }
        Err(err) => {
            warn!("Hosted login code check failed: {}", err);
            render_hosted_login_error(&domain_name)
        }
    }
}

/// 303 so the browser follows with a GET (RFC 9110 §15.4.4). The session
/// cookie is dropped: the session is done.
fn redirect_to_client(
    session: &HostedLoginSession,
    domain_name: &str,
) -> HttpResponse {
    let Some(authorization) = &session.authorization else {
        return render_hosted_login_error(domain_name);
    };

    let Some(location) = build_client_redirect(
        &session.request.redirect_uri,
        &authorization.code,
        &session.request.state,
    ) else {
        return render_hosted_login_error(domain_name);
    };

    let mut cookie = hosted_login_session_cookie("", false);
    cookie.make_removal();

    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .cookie(cookie)
        .finish()
}

fn render_code_page_with_error(
    session: &HostedLoginSession,
    domain_name: &str,
) -> HttpResponse {
    let email = session
        .email_code
        .as_ref()
        .map(|email_code| email_code.email.email())
        .unwrap_or_default();

    let mut context = TeraContext::new();
    context.insert("domain_name", domain_name);
    context.insert("csrf_token", &session.csrf_token);
    context.insert("email", &email);
    context.insert("form_action", &format!("{HOSTED_LOGIN_PATH}/code"));
    context.insert("error", "That code is not right. Check it and try again.");

    render_hosted_login_page(
        StatusCode::BAD_REQUEST,
        "web/hosted-login-code.html",
        &context,
    )
}
