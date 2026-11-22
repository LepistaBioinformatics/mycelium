use crate::domain::{
    dtos::federation::{
        FederationPeer, LinkRequest, LinkRequestSecret, PeerStatus,
    },
    entities::{FederationPeerRead, LinkRequestRegistration},
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// The open link request an initiation URL names, with its peer, without
/// spending the URL (FED-42)
///
/// Opening the URL only shows the page; a link scanner that fetches it
/// leaves the request usable.
///
#[tracing::instrument(name = "review_link_request", skip_all)]
pub async fn review_link_request(
    request_id: Uuid,
    secret: LinkRequestSecret,
    request_repo: Box<&dyn LinkRequestRegistration>,
    peer_read_repo: Box<&dyn FederationPeerRead>,
) -> Result<(LinkRequest, FederationPeer), MappedErrors> {
    let FetchResponseKind::Found(request) =
        request_repo.get_open(request_id, secret.hash()).await?
    else {
        return spent();
    };

    let peer = active_peer_of(&request, *peer_read_repo).await?;

    Ok((request, peer))
}

/// The request's peer, while it is active
pub(super) async fn active_peer_of(
    request: &LinkRequest,
    peer_read_repo: &dyn FederationPeerRead,
) -> Result<FederationPeer, MappedErrors> {
    match peer_read_repo.get(request.peer_id).await? {
        FetchResponseKind::Found(peer) if peer.status == PeerStatus::Active => {
            Ok(peer)
        }
        _ => use_case_err("the peer of this link request is not active")
            .with_exp_true()
            .as_error(),
    }
}

pub(super) fn spent<T>() -> Result<T, MappedErrors> {
    use_case_err("unknown, used or expired link request")
        .with_exp_true()
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::NewLinkRequest,
        use_cases::federation::link_test_doubles::{peer, LinkStore},
    };

    use chrono::{Duration, Local};

    async fn store_with_request(
        status: PeerStatus,
    ) -> (LinkStore, LinkRequest, LinkRequestSecret) {
        let store = LinkStore::with_peers(vec![peer(status)]);
        let secret = LinkRequestSecret::mint();
        let request = store
            .create(NewLinkRequest {
                local_user_id: Uuid::new_v4(),
                peer_id: store.peers[0].id,
                secret_hash: secret.hash(),
                expires_at: Local::now() + Duration::minutes(10),
            })
            .await
            .unwrap();

        (store, request, secret)
    }

    async fn review(
        store: &LinkStore,
        request_id: Uuid,
        secret: LinkRequestSecret,
    ) -> Result<(LinkRequest, FederationPeer), MappedErrors> {
        review_link_request(
            request_id,
            secret,
            Box::new(store),
            Box::new(store),
        )
        .await
    }

    #[tokio::test]
    async fn reviewing_shows_the_peer_and_spends_nothing() {
        let (store, request, secret) =
            store_with_request(PeerStatus::Active).await;

        for _ in 0..2 {
            let (seen, peer) =
                review(&store, request.id, secret.clone()).await.unwrap();
            assert_eq!(seen.id, request.id);
            assert_eq!(peer.id, store.peers[0].id);
        }
    }

    #[tokio::test]
    async fn a_wrong_secret_or_an_inactive_peer_is_refused() {
        let (store, request, _) = store_with_request(PeerStatus::Active).await;
        assert!(review(&store, request.id, LinkRequestSecret::mint())
            .await
            .is_err());

        let (store, request, secret) =
            store_with_request(PeerStatus::Quarantined).await;
        assert!(review(&store, request.id, secret).await.is_err());
    }
}
