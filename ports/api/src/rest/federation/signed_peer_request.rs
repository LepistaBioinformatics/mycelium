use crate::federation::PreparedFederation;

use actix_web::{
    dev::Payload, error::ErrorNotFound, web, Error, FromRequest, HttpRequest,
};
use bytes::Bytes;
use chrono::{Local, TimeZone, Utc};
use myc_core::{
    domain::{
        dtos::federation::{FederationPeer, PeerAdmission},
        entities::{
            FederationPeerRead, InstanceSettingsFetching,
            RequestNonceRegistration,
        },
    },
    use_cases::federation::{admit_requesting_peer, is_federation_enabled},
};
use myc_http_tools::{
    federation::{
        peek_peer_request_issuer, verify_peer_request, ExpectedPeerRequest,
        VerifiedPeerRequest,
    },
    responses::GatewayError,
    settings::PEER_REQUEST_SIGNATURE_KEY,
};
use std::{future::Future, pin::Pin};
use uuid::Uuid;

/// A request a declared peer signed with its federation key (FED-09, D-9)
///
/// Extracting it authenticates the caller: the signature header names a
/// declared, non-quarantined peer, the signature verifies with that peer's
/// stored keys over this method, path and body, and its `jti` was never seen
/// by any pod. The handler reads the body from here.
///
pub(crate) struct SignedPeerRequest {
    pub(crate) peer: FederationPeer,

    // Read by the handlers that take a body (resolve and tombstone, F3/F5);
    // drop the allow then.
    #[allow(dead_code)]
    pub(crate) body: Bytes,
}

impl FromRequest for SignedPeerRequest {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    #[tracing::instrument(name = "signed_peer_request_from_request", skip_all)]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let payload = payload.take();

        Box::pin(async move { authenticate(req, payload).await })
    }
}

async fn authenticate(
    req: HttpRequest,
    mut payload: Payload,
) -> Result<SignedPeerRequest, Error> {
    let (audience, tolerance_secs) = federation_context(&req).await?;
    let jws = signature_header(&req)?;
    let peer = admitted_peer(&req, &jws).await?;
    let body = Bytes::from_request(&req, &mut payload).await?;
    let expected = ExpectedPeerRequest {
        audience,
        method: req.method().as_str(),
        path: req.path(),
        body: &body,
        now: Utc::now().timestamp().max(0) as u64,
        tolerance_secs,
    };
    let verified = verify(&req, &jws, &peer, &expected).await?;
    claim_once(&req, &peer, &verified, tolerance_secs).await?;

    Ok(SignedPeerRequest { peer, body })
}

fn unauthorized(reason: &str) -> Error {
    GatewayError::Unauthorized(reason.to_string()).into()
}

fn internal(reason: impl ToString) -> Error {
    GatewayError::InternalServerError(reason.to_string()).into()
}

fn app_data<T: ?Sized + 'static>(
    req: &HttpRequest,
) -> Result<&web::Data<T>, Error> {
    req.app_data::<web::Data<T>>()
        .ok_or_else(|| internal("federation is not wired"))
}

/// This instance's id and the clock tolerance; 404 while federation is
/// absent or switched off
async fn federation_context(req: &HttpRequest) -> Result<(Uuid, u64), Error> {
    let Some(prepared) = app_data::<Option<PreparedFederation>>(req)?.get_ref()
    else {
        return Err(ErrorNotFound("federation is not enabled"));
    };
    let settings = app_data::<dyn InstanceSettingsFetching>(req)?;

    match is_federation_enabled(Box::new(settings.get_ref())).await {
        Ok(true) => {
            Ok((prepared.instance_id, prepared.config.request_tolerance_secs))
        }
        Ok(false) => Err(ErrorNotFound("federation is not enabled")),
        Err(err) => Err(internal(err)),
    }
}

fn signature_header(req: &HttpRequest) -> Result<String, Error> {
    req.headers()
        .get(PEER_REQUEST_SIGNATURE_KEY)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .ok_or_else(|| unauthorized("missing peer request signature"))
}

/// The declared peer the request claims to come from, before any key lookup
async fn admitted_peer(
    req: &HttpRequest,
    jws: &str,
) -> Result<FederationPeer, Error> {
    let (issuer, _) = peek_peer_request_issuer(jws)
        .map_err(|rejection| unauthorized(&rejection.to_string()))?;
    let peers = app_data::<dyn FederationPeerRead>(req)?;

    match admit_requesting_peer(issuer, Box::new(peers.get_ref()))
        .await
        .map_err(internal)?
    {
        PeerAdmission::Admitted(peer) => Ok(*peer),
        PeerAdmission::Unknown => Err(unauthorized("unknown peer")),
        PeerAdmission::Refused(status) => {
            Err(GatewayError::Forbidden(format!("peer is {status}")).into())
        }
    }
}

async fn verify(
    req: &HttpRequest,
    jws: &str,
    peer: &FederationPeer,
    expected: &ExpectedPeerRequest<'_>,
) -> Result<VerifiedPeerRequest, Error> {
    let peers = app_data::<dyn FederationPeerRead>(req)?;
    let keys = peers.list_keys(peer.id).await.map_err(internal)?;

    verify_peer_request(jws, &keys, expected).map_err(|rejection| {
        tracing::warn!(peer = %peer.instance_id, %rejection, "peer request refused");
        unauthorized(&rejection.to_string())
    })
}

/// Single use across every pod; the nonce is kept past `exp` + tolerance
async fn claim_once(
    req: &HttpRequest,
    peer: &FederationPeer,
    verified: &VerifiedPeerRequest,
    tolerance: u64,
) -> Result<(), Error> {
    let nonces = app_data::<dyn RequestNonceRegistration>(req)?;
    let keep_until = (verified.claims.exp + tolerance).min(i64::MAX as u64);
    let expires_at = Local
        .timestamp_opt(keep_until as i64, 0)
        .single()
        .ok_or_else(|| unauthorized("invalid exp"))?;

    match nonces
        .claim(verified.claims.jti.clone(), peer.id, expires_at)
        .await
        .map_err(internal)?
    {
        true => Ok(()),
        false => Err(unauthorized("replayed peer request")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::federation::peer_request_test_doubles::{
        federation_world, FederationWorld,
    };

    use actix_web::{http::StatusCode, test, App, HttpResponse};
    use myc_core::domain::dtos::federation::PeerStatus;

    const PATH: &str = "/federation/echo";

    async fn echo(request: SignedPeerRequest) -> HttpResponse {
        HttpResponse::Ok().body(format!(
            "{} {}",
            request.peer.instance_id,
            String::from_utf8_lossy(&request.body)
        ))
    }

    async fn call(
        world: &FederationWorld,
        jws: &str,
        body: &'static [u8],
    ) -> (StatusCode, String) {
        let app = test::init_service(
            App::new()
                .configure(|config| world.configure(config))
                .route(PATH, web::post().to(echo)),
        )
        .await;
        let request = test::TestRequest::post()
            .uri(PATH)
            .insert_header((PEER_REQUEST_SIGNATURE_KEY, jws))
            .set_payload(body)
            .to_request();
        let response = test::call_service(&app, request).await;
        let status = response.status();
        let body = test::read_body(response).await;

        (status, String::from_utf8_lossy(&body).to_string())
    }

    #[actix_web::test]
    async fn a_valid_request_reaches_the_handler_with_its_peer() {
        let world = federation_world(PeerStatus::Active);
        let issuer = world.peers.peer.instance_id;
        let jws = world.sign(issuer, PATH, "jti-valid", b"{\"x\":1}");

        let (status, body) = call(&world, &jws, b"{\"x\":1}").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, format!("{issuer} {{\"x\":1}}"));
    }

    #[actix_web::test]
    async fn an_unknown_peer_is_refused_without_a_key_lookup() {
        let world = federation_world(PeerStatus::Active);
        let jws = world.sign(Uuid::new_v4(), PATH, "jti-unknown", b"{}");

        let (status, _) = call(&world, &jws, b"{}").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(world.peers.key_reads(), 0);
    }

    #[actix_web::test]
    async fn a_quarantined_peer_is_forbidden() {
        let world = federation_world(PeerStatus::Quarantined);
        let issuer = world.peers.peer.instance_id;
        let jws = world.sign(issuer, PATH, "jti-quarantined", b"{}");

        let (status, _) = call(&world, &jws, b"{}").await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(world.peers.key_reads(), 0);
    }

    #[actix_web::test]
    async fn a_replayed_jti_is_refused() {
        let world = federation_world(PeerStatus::Active);
        let issuer = world.peers.peer.instance_id;
        let jws = world.sign(issuer, PATH, "jti-replayed", b"{}");

        assert_eq!(call(&world, &jws, b"{}").await.0, StatusCode::OK);
        assert_eq!(call(&world, &jws, b"{}").await.0, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn a_body_other_than_the_signed_one_is_refused() {
        let world = federation_world(PeerStatus::Active);
        let issuer = world.peers.peer.instance_id;
        let jws = world.sign(issuer, PATH, "jti-body", b"{\"x\":1}");

        let (status, _) = call(&world, &jws, b"{\"x\":2}").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn a_missing_signature_is_unauthorized() {
        let world = federation_world(PeerStatus::Active);

        let (status, _) = call(&world, "", b"{}").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn the_kill_switch_refuses_every_request() {
        let world = FederationWorld {
            enabled: false,
            ..federation_world(PeerStatus::Active)
        };
        let issuer = world.peers.peer.instance_id;
        let jws = world.sign(issuer, PATH, "jti-killed", b"{}");

        let (status, _) = call(&world, &jws, b"{}").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(world.peers.key_reads(), 0);
    }
}
