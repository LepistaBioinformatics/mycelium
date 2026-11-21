use crate::domain::{
    dtos::federation::{PeerAdmission, PeerStatus},
    entities::FederationPeerRead,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

/// Decide whether the caller named by a request's `iss` may be served
///
/// Being listed by the caller grants nothing: only this instance's own peer
/// list counts (FED-09). Quarantined and untrusted peers are refused.
///
#[tracing::instrument(name = "admit_requesting_peer", skip_all)]
pub async fn admit_requesting_peer(
    issuer: Uuid,
    peer_read_repo: Box<&dyn FederationPeerRead>,
) -> Result<PeerAdmission, MappedErrors> {
    let FetchResponseKind::Found(peer) =
        peer_read_repo.get_by_instance_id(issuer).await?
    else {
        return Ok(PeerAdmission::Unknown);
    };

    match peer.status {
        PeerStatus::Quarantined | PeerStatus::Untrusted => {
            Ok(PeerAdmission::Refused(peer.status))
        }
        _ => Ok(PeerAdmission::Admitted(Box::new(peer))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::federation::{
        FederationPeer, FederationPolicy, PeerKey,
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::FetchManyResponseKind;

    struct Peers(Option<FederationPeer>);

    #[async_trait]
    impl FederationPeerRead for Peers {
        async fn get(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors>
        {
            unreachable!()
        }

        async fn get_by_instance_id(
            &self,
            instance_id: Uuid,
        ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors>
        {
            Ok(match &self.0 {
                Some(peer) if peer.instance_id == instance_id => {
                    FetchResponseKind::Found(peer.clone())
                }
                _ => FetchResponseKind::NotFound(Some(instance_id)),
            })
        }

        async fn list(
            &self,
        ) -> Result<FetchManyResponseKind<FederationPeer>, MappedErrors>
        {
            unreachable!()
        }

        async fn list_keys(
            &self,
            _: Uuid,
        ) -> Result<Vec<PeerKey>, MappedErrors> {
            unreachable!("admission never reads keys")
        }
    }

    fn peer(status: PeerStatus) -> FederationPeer {
        FederationPeer {
            id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            base_url: "https://myc-02.example.com".to_string(),
            display_name: None,
            status,
            consume_kinds: vec![],
            serve_kinds: vec![],
            negotiated_kinds: vec![],
            policy: FederationPolicy::default(),
            last_contact_at: None,
            failure_count: 0,
            created: Local::now(),
            created_by: None,
            updated: None,
            updated_by: None,
        }
    }

    async fn admit(
        peers: &Peers,
        issuer: Uuid,
    ) -> Result<PeerAdmission, MappedErrors> {
        admit_requesting_peer(
            issuer,
            Box::new(peers as &dyn FederationPeerRead),
        )
        .await
    }

    #[tokio::test]
    async fn an_undeclared_caller_is_unknown() -> Result<(), MappedErrors> {
        assert_eq!(
            admit(&Peers(None), Uuid::new_v4()).await?,
            PeerAdmission::Unknown
        );

        Ok(())
    }

    #[tokio::test]
    async fn quarantined_and_untrusted_peers_are_refused(
    ) -> Result<(), MappedErrors> {
        for status in [PeerStatus::Quarantined, PeerStatus::Untrusted] {
            let declared = peer(status);
            let issuer = declared.instance_id;

            assert_eq!(
                admit(&Peers(Some(declared)), issuer).await?,
                PeerAdmission::Refused(status)
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn other_declared_peers_are_admitted() -> Result<(), MappedErrors> {
        for status in [
            PeerStatus::Declared,
            PeerStatus::Handshaking,
            PeerStatus::Active,
            PeerStatus::Degraded,
            PeerStatus::Unreachable,
        ] {
            let declared = peer(status);
            let issuer = declared.instance_id;

            assert_eq!(
                admit(&Peers(Some(declared.clone())), issuer).await?,
                PeerAdmission::Admitted(Box::new(declared))
            );
        }

        Ok(())
    }
}
