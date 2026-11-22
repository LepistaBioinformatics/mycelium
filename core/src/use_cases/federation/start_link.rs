use crate::domain::{
    dtos::federation::{
        FederationPeer, LinkRequest, LinkRequestSecret, NewLinkRequest,
        PeerStatus,
    },
    entities::{FederationPeerRead, IdentityLinkRead, LinkRequestRegistration},
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Open a single-use, short-lived link initiation towards an active peer
/// (FED-42)
///
/// Returns the request and the URL secret; the caller builds the initiation
/// URL. Only the secret's hash is stored.
///
#[tracing::instrument(name = "start_link", skip_all)]
pub async fn start_link(
    local_user_id: Uuid,
    peer_instance_id: Uuid,
    lifetime: Duration,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    request_repo: Box<&dyn LinkRequestRegistration>,
) -> Result<(LinkRequest, LinkRequestSecret), MappedErrors> {
    let peer = active_peer(peer_instance_id, *peer_read_repo).await?;

    if let FetchResponseKind::Found(_) =
        link_read_repo.find_live(local_user_id, peer.id).await?
    {
        return use_case_err("the account is already linked to this peer")
            .with_exp_true()
            .as_error();
    }

    let secret = LinkRequestSecret::mint();
    let request = request_repo
        .create(NewLinkRequest {
            local_user_id,
            peer_id: peer.id,
            secret_hash: secret.hash(),
            expires_at: Local::now() + lifetime,
        })
        .await?;

    Ok((request, secret))
}

async fn active_peer(
    instance_id: Uuid,
    peer_read_repo: &dyn FederationPeerRead,
) -> Result<FederationPeer, MappedErrors> {
    match peer_read_repo.get_by_instance_id(instance_id).await? {
        FetchResponseKind::Found(peer) if peer.status == PeerStatus::Active => {
            Ok(peer)
        }
        _ => use_case_err("no active peer with this instance id")
            .with_exp_true()
            .as_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            dtos::federation::{LinkStatus, NewIdentityLink},
            entities::IdentityLinkWrite,
        },
        use_cases::federation::link_test_doubles::{peer, LinkStore},
    };

    async fn start(
        store: &LinkStore,
        user_id: Uuid,
        instance_id: Uuid,
    ) -> Result<(LinkRequest, LinkRequestSecret), MappedErrors> {
        start_link(
            user_id,
            instance_id,
            Duration::minutes(10),
            Box::new(store),
            Box::new(store),
            Box::new(store),
        )
        .await
    }

    #[tokio::test]
    async fn an_active_peer_gets_a_request_storing_only_the_hash() {
        let active = peer(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![active.clone()]);
        let user_id = Uuid::new_v4();

        let (request, secret) =
            start(&store, user_id, active.instance_id).await.unwrap();

        let stored = store.requests.lock().unwrap();
        assert_eq!(request.peer_id, active.id);
        assert_eq!(request.local_user_id, user_id);
        assert!(request.expires_at > Local::now());
        assert!(secret.matches(&stored[0].1.secret_hash));
        assert_ne!(stored[0].1.secret_hash, secret.as_str());
    }

    #[tokio::test]
    async fn an_unknown_quarantined_or_degraded_peer_is_refused() {
        let held = peer(PeerStatus::Quarantined);
        let degraded = peer(PeerStatus::Degraded);
        let store = LinkStore::with_peers(vec![held.clone(), degraded.clone()]);

        for instance_id in
            [held.instance_id, degraded.instance_id, Uuid::new_v4()]
        {
            assert!(start(&store, Uuid::new_v4(), instance_id).await.is_err());
        }
        assert!(store.requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_user_already_linked_to_the_peer_is_refused() {
        let active = peer(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![active.clone()]);
        let user_id = Uuid::new_v4();
        store
            .record(NewIdentityLink {
                local_user_id: user_id,
                local_email: "user@myc-01.example.com".to_string(),
                peer_id: active.id,
                peer_subject_ref: None,
                status: LinkStatus::Active,
            })
            .await
            .unwrap();

        assert!(start(&store, user_id, active.instance_id).await.is_err());
    }
}
