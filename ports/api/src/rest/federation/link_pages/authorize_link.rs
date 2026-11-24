use super::{
    enabled_federation::enabled_federation, render_link_page::render_link_error,
};
use crate::{
    federation::{link_callback_url, PreparedFederation, LINK_LOGIN_CLIENT_ID},
    models::active_backend_modules::SqlAppModule,
    rest::{
        federation::verify_with_peer_keys::verify_with_peer_keys,
        hosted_login::HOSTED_LOGIN_PATH,
    },
};

use actix_web::{http::header::LOCATION, web, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use myc_core::{
    domain::{
        dtos::federation::{FederationPeer, LinkConsent},
        entities::FederationPeerRead,
    },
    use_cases::federation::{open_link_consent, ConsentAsked},
};
use myc_http_tools::federation::{
    derive_link_secret, peek_peer_request_issuer, pkce_challenge,
    verify_link_state, ExpectedLinkState, LinkSecretPurpose, VerifiedLinkState,
};
use mycelium_base::entities::FetchResponseKind;
use serde::Deserialize;
use shaku::HasComponent;
use url::form_urlencoded;

const REFUSED: &str = "This link request is invalid, was already used or \
                       comes from an instance this one does not trust. Start \
                       again from the application.";

#[derive(Deserialize)]
pub(crate) struct AuthorizeLinkQuery {
    state: String,
}

/// A consumer sends a linking user here with its signed state (FED-46).
/// Everything is checked before any login is offered (failure case 26).
#[tracing::instrument(name = "authorize_link", skip_all)]
pub(crate) async fn authorize_link(
    req: HttpRequest,
    query: web::Query<AuthorizeLinkQuery>,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(prepared) = enabled_federation(&prepared, &sql_module).await
    else {
        return HttpResponse::NotFound().finish();
    };
    let Some(peer) = issuing_peer(&query.state, &sql_module).await else {
        return render_link_error(REFUSED);
    };
    let Ok(verified) =
        verified_state(&req, prepared, &peer, &query.state).await
    else {
        return render_link_error(REFUSED);
    };

    match open_link_consent(
        &peer,
        ConsentAsked {
            request_id: verified.claims.jti,
            peer_email: verified.claims.email,
            return_url: verified.claims.return_url,
        },
        Duration::seconds(prepared.config.link_request_ttl_secs as i64),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    {
        Ok(consent) => send_to_hosted_login(prepared, &consent),
        Err(_) => render_link_error(REFUSED),
    }
}

/// The declared peer the state claims to come from
async fn issuing_peer(
    state: &str,
    sql_module: &SqlAppModule,
) -> Option<FederationPeer> {
    let (issuer, _) = peek_peer_request_issuer(state).ok()?;
    let peers: &dyn FederationPeerRead = sql_module.resolve_ref();

    match peers.get_by_instance_id(issuer).await.ok()? {
        FetchResponseKind::Found(peer) => Some(peer),
        FetchResponseKind::NotFound(_) => None,
    }
}

async fn verified_state(
    req: &HttpRequest,
    prepared: &PreparedFederation,
    peer: &FederationPeer,
    state: &str,
) -> Result<VerifiedLinkState, actix_web::Error> {
    let expected = ExpectedLinkState {
        audience: prepared.instance_id,
        now: Utc::now().timestamp().max(0) as u64,
        tolerance_secs: prepared.config.request_tolerance_secs,
    };

    verify_with_peer_keys(req, peer, |keys| {
        verify_link_state(state, keys, &expected)
    })
    .await
}

/// Sign the user in through this instance's hosted login, as the internal
/// client, with a PKCE challenge only this instance can answer
fn send_to_hosted_login(
    prepared: &PreparedFederation,
    consent: &LinkConsent,
) -> HttpResponse {
    let Ok(verifier) = derive_link_secret(
        &prepared.signing.primary,
        LinkSecretPurpose::Pkce,
        consent.id,
    ) else {
        return HttpResponse::InternalServerError().finish();
    };

    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("response_type", "code")
        .append_pair("client_id", LINK_LOGIN_CLIENT_ID)
        .append_pair("redirect_uri", &link_callback_url(prepared))
        .append_pair("state", &consent.id.to_string())
        .append_pair("code_challenge", &pkce_challenge(&verifier))
        .append_pair("code_challenge_method", "S256")
        .finish();

    HttpResponse::SeeOther()
        .insert_header((
            LOCATION,
            format!("{HOSTED_LOGIN_PATH}/authorize?{query}"),
        ))
        .finish()
}
