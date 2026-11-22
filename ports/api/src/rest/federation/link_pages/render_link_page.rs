use crate::rest::hosted_page_response::hosted_page_response;

use actix_web::{
    http::{
        header::{HeaderValue, CACHE_CONTROL, REFERRER_POLICY},
        StatusCode,
    },
    HttpResponse,
};
use myc_core::settings::TEMPLATES;
use tera::Context as TeraContext;
use tracing::warn;

/// Render a linking page: never framed, never cached, and the URL (which
/// carries the initiation secret or the consent) never leaves as a referrer.
/// `same-origin` rather than `no-referrer`, so the page's own form POST keeps
/// its `Origin` (see the hosted login pages).
pub(super) fn render_link_page(
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

/// The flow cannot continue. Never redirects: the redirect target may be
/// what failed validation.
pub(super) fn render_link_error(message: &str) -> HttpResponse {
    let mut context = TeraContext::new();
    context.insert("message", message);

    render_link_page(
        StatusCode::BAD_REQUEST,
        "web/federation-link-error.html",
        &context,
    )
}
