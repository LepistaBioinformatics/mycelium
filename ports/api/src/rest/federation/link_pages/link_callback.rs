use super::{
    consumer_link_pages::{instance_name, peer_name},
    enabled_federation::enabled_federation,
    render_link_page::{render_link_error, render_link_page},
};
use crate::{
    federation::{link_callback_url, PreparedFederation, LINK_LOGIN_CLIENT_ID},
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{http::StatusCode, web, HttpResponse};
use myc_core::{
    domain::dtos::{
        federation::{FederationPeer, LinkConsent},
        user::User,
    },
    use_cases::{
        federation::{accept_link_login, LinkingUser},
        role_scoped::beginner::hosted_login::exchange_hosted_login_code,
    },
};
use myc_http_tools::{
    federation::{derive_link_secret, LinkSecretPurpose},
    settings::FEDERATION_LINK_APPROVE_PATH,
};
use serde::Deserialize;
use shaku::HasComponent;
use tera::Context as TeraContext;
use uuid::Uuid;

const REFUSED: &str = "The sign-in did not complete, the accounts' emails \
                       differ, or this link was already used. Start again \
                       from the application.";

#[derive(Deserialize)]
pub(crate) struct LinkCallbackQuery {
    code: String,
    state: Uuid,
}

/// The hosted login brings the linking user back: exchange the code here,
/// apply the same-email rule and ask for confirmation (FED-44 … FED-46)
#[tracing::instrument(name = "link_callback", skip_all)]
pub(crate) async fn link_callback(
    query: web::Query<LinkCallbackQuery>,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(prepared) = enabled_federation(&prepared, &sql_module).await
    else {
        return HttpResponse::NotFound().finish();
    };
    let query = query.into_inner();
    let Some(user) = signed_in_user(prepared, &query, &sql_module).await else {
        return render_link_error(REFUSED);
    };
    let (Some(id), email) = (user.id, user.email.email()) else {
        return render_link_error(REFUSED);
    };

    match accept_link_login(
        query.state,
        LinkingUser {
            id,
            email: email.clone(),
        },
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    {
        Ok((consent, peer)) => confirm_page(prepared, &consent, &peer, &email),
        Err(_) => render_link_error(REFUSED),
    }
}

async fn signed_in_user(
    prepared: &PreparedFederation,
    query: &LinkCallbackQuery,
    sql_module: &SqlAppModule,
) -> Option<User> {
    let verifier = derive_link_secret(
        &prepared.signing.primary,
        LinkSecretPurpose::Pkce,
        query.state,
    )
    .ok()?;

    exchange_hosted_login_code(
        LINK_LOGIN_CLIENT_ID.to_string(),
        link_callback_url(prepared),
        query.code.clone(),
        verifier,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    .ok()
}

/// Names the local account, the peer's account and the peer (FED-46)
fn confirm_page(
    prepared: &PreparedFederation,
    consent: &LinkConsent,
    peer: &FederationPeer,
    local_email: &str,
) -> HttpResponse {
    let Ok(csrf_token) = derive_link_secret(
        &prepared.signing.primary,
        LinkSecretPurpose::Csrf,
        consent.id,
    ) else {
        return HttpResponse::InternalServerError().finish();
    };

    let mut context = TeraContext::new();
    context.insert("instance_name", &instance_name(prepared));
    context.insert("peer_name", &peer_name(peer));
    context.insert("local_email", local_email);
    context.insert("peer_email", &consent.peer_email);
    context.insert("consent", &consent.id.to_string());
    context.insert("csrf_token", &csrf_token);
    context.insert("action", FEDERATION_LINK_APPROVE_PATH);

    render_link_page(
        StatusCode::OK,
        "web/federation-link-confirm.html",
        &context,
    )
}
