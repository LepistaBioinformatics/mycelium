use crate::federation::{describe_this_instance, PreparedFederation};

use actix_web::{web, Error, HttpRequest};
use myc_core::{
    domain::{
        dtos::federation::FederationPeer,
        entities::{FederationPeerWrite, PeerClient},
    },
    use_cases::federation::handshake_peer,
};
use myc_http_tools::responses::GatewayError;

/// Handshake with the peer now, to fetch keys it published since the last
/// round (spec failure case 12)
///
/// The handshake re-checks the peer's `instanceId`, so a peer that now
/// announces another one becomes untrusted instead of being re-pinned.
///
pub(super) async fn refresh_peer_keys(
    req: &HttpRequest,
    peer: &FederationPeer,
) -> Result<(), Error> {
    let (prepared, client, write) = wiring(req)?;

    handshake_peer(
        peer,
        &describe_this_instance(&prepared, false).assertion_kinds,
        prepared.config.max_failures_before_open,
        Box::new(client.get_ref()),
        Box::new(write.get_ref()),
    )
    .await
    .map(|_| ())
    .map_err(|err| GatewayError::InternalServerError(err.to_string()).into())
}

type Wiring = (
    PreparedFederation,
    web::Data<dyn PeerClient>,
    web::Data<dyn FederationPeerWrite>,
);

fn wiring(req: &HttpRequest) -> Result<Wiring, Error> {
    let prepared = req
        .app_data::<web::Data<Option<PreparedFederation>>>()
        .and_then(|data| data.get_ref().clone());
    let client = req.app_data::<web::Data<dyn PeerClient>>().cloned();
    let write = req
        .app_data::<web::Data<dyn FederationPeerWrite>>()
        .cloned();

    match (prepared, client, write) {
        (Some(prepared), Some(client), Some(write)) => {
            Ok((prepared, client, write))
        }
        _ => Err(GatewayError::InternalServerError(
            "federation is not wired".to_string(),
        )
        .into()),
    }
}
