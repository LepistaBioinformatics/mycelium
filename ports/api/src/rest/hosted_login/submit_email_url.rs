use super::{
    hosted_login_session_cookie::read_hosted_login_session_id,
    render_hosted_login_page::{
        domain_name, render_hosted_login_error, render_hosted_login_page,
    },
    HOSTED_LOGIN_PATH,
};
use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{
    http::StatusCode, post, web, HttpRequest, HttpResponse, Responder,
};
use myc_core::{
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::hosted_login::send_hosted_login_code,
};
use myc_http_tools::Email;
use serde::Deserialize;
use shaku::HasComponent;
use tera::Context as TeraContext;
use tracing::warn;

#[derive(Deserialize)]
pub struct EmailForm {
    csrf_token: String,
    email: String,
}

/// Email a code to the address and render the page asking for it.
#[post("/email")]
pub async fn submit_email_url(
    req: HttpRequest,
    form: web::Form<EmailForm>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let domain_name = domain_name(&life_cycle_settings).await;
    let form = form.into_inner();

    let Some(session_id) = read_hosted_login_session_id(&req) else {
        return render_hosted_login_error(&domain_name);
    };

    let Ok(email) = Email::from_string(form.email.trim().to_owned()) else {
        return render_email_page_with_error(&domain_name, &form.csrf_token);
    };

    let session = match send_hosted_login_code(
        session_id,
        form.csrf_token,
        email.clone(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(session) => session,
        Err(err) => {
            warn!("Hosted login code not sent: {}", err);
            return render_hosted_login_error(&domain_name);
        }
    };

    let mut context = TeraContext::new();
    context.insert("domain_name", &domain_name);
    context.insert("csrf_token", &session.csrf_token);
    context.insert("email", &email.email());
    context.insert("form_action", &format!("{HOSTED_LOGIN_PATH}/code"));

    render_hosted_login_page(
        StatusCode::OK,
        "web/hosted-login-code.html",
        &context,
    )
}

fn render_email_page_with_error(
    domain_name: &str,
    csrf_token: &str,
) -> HttpResponse {
    let mut context = TeraContext::new();
    context.insert("domain_name", domain_name);
    context.insert("csrf_token", csrf_token);
    context.insert("form_action", &format!("{HOSTED_LOGIN_PATH}/email"));
    context.insert("error", "Enter a valid email address.");

    render_hosted_login_page(
        StatusCode::BAD_REQUEST,
        "web/hosted-login-email.html",
        &context,
    )
}
