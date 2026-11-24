use super::{
    enabled_federation::enabled_federation,
    link_request_token::read_link_request_token,
    render_link_page::{render_link_error, render_link_page},
};
use crate::{
    federation::PreparedFederation,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{
    http::{header::LOCATION, StatusCode},
    web, HttpResponse,
};
use chrono::Utc;
use myc_core::{
    domain::dtos::federation::FederationPeer,
    use_cases::federation::{
        redirect_link_request, review_link_request, LinkRedirect,
    },
};
use myc_http_tools::{
    federation::{sign_link_state, LinkStateClaims},
    settings::{
        FEDERATION_LINK_AUTHORIZE_PATH, FEDERATION_LINK_DONE_PATH,
        FEDERATION_LINK_START_PATH,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use tera::Context as TeraContext;
use tracing::warn;

const SPENT: &str = "This link request is unknown, was already used or has \
                     expired. Start again from the application.";

#[derive(Deserialize)]
pub(crate) struct LinkRequestForm {
    request: String,
}

/// The page an initiation URL opens: names the peer and asks to continue.
/// Reading it spends nothing (FED-42).
#[tracing::instrument(name = "show_link_start", skip_all)]
pub(crate) async fn show_link_start(
    query: web::Query<LinkRequestForm>,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(prepared) = enabled_federation(&prepared, &sql_module).await
    else {
        return HttpResponse::NotFound().finish();
    };
    let Some((request_id, secret)) = read_link_request_token(&query.request)
    else {
        return render_link_error(SPENT);
    };

    match review_link_request(
        request_id,
        secret,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    {
        Ok((_, peer)) => start_page(prepared, &peer, &query.request),
        Err(_) => render_link_error(SPENT),
    }
}

fn start_page(
    prepared: &PreparedFederation,
    peer: &FederationPeer,
    token: &str,
) -> HttpResponse {
    let mut context = TeraContext::new();
    context.insert("instance_name", &instance_name(prepared));
    context.insert("peer_name", &peer_name(peer));
    context.insert("request", token);
    context.insert("action", FEDERATION_LINK_START_PATH);

    render_link_page(StatusCode::OK, "web/federation-link-start.html", &context)
}

/// Spend the initiation URL and send the browser to the issuer with the
/// signed state (FED-46)
#[tracing::instrument(name = "continue_link_start", skip_all)]
pub(crate) async fn continue_link_start(
    form: web::Form<LinkRequestForm>,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(prepared) = enabled_federation(&prepared, &sql_module).await
    else {
        return HttpResponse::NotFound().finish();
    };
    let Some((request_id, secret)) = read_link_request_token(&form.request)
    else {
        return render_link_error(SPENT);
    };

    match redirect_link_request(
        request_id,
        secret,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    {
        Ok(redirect) => redirect_to_issuer(prepared, redirect),
        Err(_) => render_link_error(SPENT),
    }
}

fn redirect_to_issuer(
    prepared: &PreparedFederation,
    (request, peer, email): LinkRedirect,
) -> HttpResponse {
    let claims = LinkStateClaims {
        iss: prepared.instance_id,
        aud: peer.instance_id,
        iat: Utc::now().timestamp().max(0) as u64,
        exp: request.expires_at.timestamp().max(0) as u64,
        jti: request.id,
        email,
        return_url: format!(
            "{}{FEDERATION_LINK_DONE_PATH}",
            prepared.config.base_url.trim_end_matches('/')
        ),
    };

    let state = match sign_link_state(&prepared.signing.primary, &claims) {
        Ok(state) => state,
        Err(err) => {
            warn!("Failed to sign the link state: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::SeeOther()
        .insert_header((
            LOCATION,
            format!(
                "{}{FEDERATION_LINK_AUTHORIZE_PATH}?state={state}",
                peer.base_url.trim_end_matches('/')
            ),
        ))
        .finish()
}

/// Where the issuer sends the browser back once the link is recorded
#[tracing::instrument(name = "show_link_done", skip_all)]
pub(crate) async fn show_link_done(
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(prepared) = enabled_federation(&prepared, &sql_module).await
    else {
        return HttpResponse::NotFound().finish();
    };
    let mut context = TeraContext::new();
    context.insert("instance_name", &instance_name(prepared));

    render_link_page(StatusCode::OK, "web/federation-link-done.html", &context)
}

pub(super) fn instance_name(prepared: &PreparedFederation) -> String {
    prepared
        .config
        .display_name
        .clone()
        .unwrap_or_else(|| prepared.config.base_url.clone())
}

pub(super) fn peer_name(peer: &FederationPeer) -> String {
    peer.display_name
        .clone()
        .unwrap_or_else(|| peer.base_url.clone())
}
