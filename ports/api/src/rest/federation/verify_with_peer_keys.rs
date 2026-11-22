use super::{
    refresh_peer_keys::refresh_peer_keys,
    signed_peer_request::{app_data, internal, unauthorized},
};

use actix_web::{Error, HttpRequest};
use chrono::Local;
use myc_core::{
    domain::{
        dtos::federation::{FederationPeer, PeerKey},
        entities::FederationPeerRead,
    },
    use_cases::federation::peer_keys_refresh_due,
};
use myc_http_tools::federation::{
    verify_peer_request, ExpectedPeerRequest, PeerRequestRejection,
    VerifiedPeerRequest,
};
use std::time::Duration;

/// Least time between two key refreshes a request can trigger for one peer
const KEY_REFRESH_COOL_DOWN: Duration = Duration::from_secs(30);

/// Verify a signed peer request with the stored keys; an unknown `kid`
/// refreshes them once, at most every `KEY_REFRESH_COOL_DOWN`
pub(super) async fn verify(
    req: &HttpRequest,
    jws: &str,
    peer: &FederationPeer,
    expected: &ExpectedPeerRequest<'_>,
) -> Result<VerifiedPeerRequest, Error> {
    verify_with_peer_keys(req, peer, |keys| {
        verify_peer_request(jws, keys, expected)
    })
    .await
}

/// Run `check` against the peer's stored keys; when it reports an unknown
/// `kid`, refresh the keys once (at most every `KEY_REFRESH_COOL_DOWN`) and
/// run it again
pub(crate) async fn verify_with_peer_keys<T>(
    req: &HttpRequest,
    peer: &FederationPeer,
    check: impl Fn(&[PeerKey]) -> Result<T, PeerRequestRejection>,
) -> Result<T, Error> {
    let keys = stored_keys(req, peer).await?;

    match check(&keys) {
        Err(PeerRequestRejection::UnknownKid(_))
            if peer_keys_refresh_due(
                &keys,
                peer.updated,
                Local::now(),
                KEY_REFRESH_COOL_DOWN,
            ) =>
        {
            refresh_peer_keys(req, peer).await?;
            let keys = stored_keys(req, peer).await?;
            check(&keys).map_err(|r| refused(peer, r))
        }
        outcome => outcome.map_err(|r| refused(peer, r)),
    }
}

async fn stored_keys(
    req: &HttpRequest,
    peer: &FederationPeer,
) -> Result<Vec<PeerKey>, Error> {
    let peers = app_data::<dyn FederationPeerRead>(req)?;

    peers.list_keys(peer.id).await.map_err(internal)
}

fn refused(peer: &FederationPeer, rejection: PeerRequestRejection) -> Error {
    tracing::warn!(peer = %peer.instance_id, %rejection, "peer request refused");
    unauthorized(&rejection.to_string())
}
