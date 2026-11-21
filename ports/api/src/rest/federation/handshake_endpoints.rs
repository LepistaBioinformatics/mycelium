use super::SignedPeerRequest;
use crate::federation::{
    describe_this_instance, role_catalog_published, PreparedFederation,
};

use actix_web::{web, HttpResponse};
use myc_core::{
    domain::entities::{
        FederationPeerWrite, InstanceSettingsFetching, PeerClient,
    },
    use_cases::federation::handshake_peer,
};
use myc_http_tools::settings::FEDERATION_HANDSHAKE_PATH;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config.route(
        FEDERATION_HANDSHAKE_PATH,
        web::post().to(accept_peer_handshake),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// A declared peer asks this instance to handshake with it again (FED-08)
///
/// The caller is authenticated before anything else runs (FED-09). This side
/// then fetches the caller's self-description itself and answers with its
/// own. A failed callback is recorded on the peer; the answer is still this
/// instance's self-description.
///
#[tracing::instrument(name = "accept_peer_handshake", skip_all)]
pub(crate) async fn accept_peer_handshake(
    request: SignedPeerRequest,
    prepared: web::Data<Option<PreparedFederation>>,
    peer_client: web::Data<dyn PeerClient>,
    peer_write: web::Data<dyn FederationPeerWrite>,
    settings: web::Data<dyn InstanceSettingsFetching>,
) -> HttpResponse {
    let Some(prepared) = prepared.get_ref() else {
        return HttpResponse::NotFound().finish();
    };
    let published = role_catalog_published(settings.get_ref()).await;
    let description = describe_this_instance(prepared, published);

    if let Err(err) = handshake_peer(
        &request.peer,
        &description.assertion_kinds,
        prepared.config.max_failures_before_open,
        Box::new(peer_client.get_ref()),
        Box::new(peer_write.get_ref()),
    )
    .await
    {
        tracing::error!(%err, peer = %request.peer.instance_id, "handshake failed");
    }

    HttpResponse::Ok().json(description)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::federation::peer_request_test_doubles::{
        federation_world, FederationWorld,
    };

    use actix_web::{http::StatusCode, test, App};
    use myc_core::domain::dtos::federation::PeerStatus;
    use myc_http_tools::settings::PEER_REQUEST_SIGNATURE_KEY;
    use serde_json::Value;
    use uuid::Uuid;

    async fn call(world: &FederationWorld, jws: &str) -> (StatusCode, Value) {
        let app = test::init_service(
            App::new()
                .configure(|config| world.configure(config))
                .configure(configure),
        )
        .await;
        let request = test::TestRequest::post()
            .uri(FEDERATION_HANDSHAKE_PATH)
            .insert_header((PEER_REQUEST_SIGNATURE_KEY, jws))
            .to_request();
        let response = test::call_service(&app, request).await;
        let status = response.status();
        let body = test::read_body(response).await;

        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    #[actix_web::test]
    async fn an_undeclared_caller_is_refused_before_any_handshake_work() {
        let world = federation_world(PeerStatus::Active);
        let jws =
            world.sign(Uuid::new_v4(), FEDERATION_HANDSHAKE_PATH, "j-1", b"");

        let (status, _) = call(&world, &jws).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(world.remote.calls(), 0);
        assert!(world.peers.statuses.lock().unwrap().is_empty());
    }

    #[actix_web::test]
    async fn a_declared_peer_gets_this_instance_and_a_fresh_handshake() {
        let world = federation_world(PeerStatus::Declared);
        let issuer = world.peers.peer.instance_id;
        let jws = world.sign(issuer, FEDERATION_HANDSHAKE_PATH, "j-2", b"");

        let (status, body) = call(&world, &jws).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["instanceId"], world.local.instance_id.to_string());
        assert_eq!(world.remote.calls(), 1);
        assert_eq!(*world.peers.statuses.lock().unwrap(), [PeerStatus::Active]);
    }
}
