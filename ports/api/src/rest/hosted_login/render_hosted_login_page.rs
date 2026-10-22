use crate::rest::hosted_page_response::hosted_page_response;

use actix_web::{
    http::{
        header::{HeaderValue, CACHE_CONTROL, REFERRER_POLICY},
        StatusCode,
    },
    web, HttpResponse,
};
use myc_core::{models::AccountLifeCycle, settings::TEMPLATES};
use tera::Context as TeraContext;
use tracing::warn;

pub(super) async fn domain_name(
    life_cycle_settings: &web::Data<AccountLifeCycle>,
) -> String {
    life_cycle_settings
        .domain_name
        .async_get_or_error()
        .await
        .unwrap_or_default()
}

/// Render a hosted login page. Pages carry the CSRF token and the session
/// flow, so they are never cached and never leak the URL as a referrer to
/// other sites. `same-origin`, not `no-referrer`: with `no-referrer` the
/// browser sends `Origin: null` on the page's own form POSTs, which the CORS
/// middleware rejects.
pub(super) fn render_hosted_login_page(
    status: StatusCode,
    template: &str,
    context: &TeraContext,
) -> HttpResponse {
    let html = match TEMPLATES.render(template, context) {
        Ok(html) => html,
        Err(err) => {
            warn!("Failed to render {} template: {}", template, err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut response = hosted_page_response(status, html);
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("same-origin"));

    response
}

/// The flow cannot continue: the client, the session or the code is no
/// longer valid. Never redirects, since the redirect target may be the
/// thing that failed validation.
pub(super) fn render_hosted_login_error(domain_name: &str) -> HttpResponse {
    render_hosted_login_error_with(domain_name, None)
}

/// The client's authorization request itself was refused: unknown client,
/// unregistered redirect URI, or missing/invalid OAuth parameters.
pub(super) fn render_invalid_authorization_request(
    domain_name: &str,
) -> HttpResponse {
    render_hosted_login_error_with(
        domain_name,
        Some(
            "This application is not allowed to start a sign-in here, or the \
             sign-in link is missing required parameters.",
        ),
    )
}

fn render_hosted_login_error_with(
    domain_name: &str,
    message: Option<&str>,
) -> HttpResponse {
    let mut context = TeraContext::new();
    context.insert("domain_name", domain_name);
    context.insert("message", &message);

    render_hosted_login_page(
        StatusCode::BAD_REQUEST,
        "web/hosted-login-error.html",
        &context,
    )
}
