use crate::domain::{
    dtos::federation::PeerOverview, entities::FederationPeerRead,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// Every declared peer with the `kid`s stored for it (FED-33)
#[tracing::instrument(name = "list_federation_peers", skip_all)]
pub async fn list_federation_peers(
    peer_read_repo: Box<&dyn FederationPeerRead>,
) -> Result<Vec<PeerOverview>, MappedErrors> {
    let FetchManyResponseKind::Found(peers) = peer_read_repo.list().await?
    else {
        return Ok(vec![]);
    };

    let mut overviews = Vec::with_capacity(peers.len());
    for peer in peers {
        let key_ids = peer_read_repo
            .list_keys(peer.id)
            .await?
            .into_iter()
            .map(|key| key.kid)
            .collect();

        overviews.push(PeerOverview { peer, key_ids });
    }

    Ok(overviews)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::federation::{
        FederationPeer, FederationPolicy, PeerKey, PeerStatus,
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::FetchResponseKind;
    use uuid::Uuid;

    struct Listed(Vec<FederationPeer>);

    #[async_trait]
    impl FederationPeerRead for Listed {
        async fn get(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors>
        {
            unreachable!()
        }

        async fn get_by_instance_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors>
        {
            unreachable!()
        }

        async fn list(
            &self,
        ) -> Result<FetchManyResponseKind<FederationPeer>, MappedErrors>
        {
            match self.0.is_empty() {
                true => Ok(FetchManyResponseKind::NotFound),
                false => Ok(FetchManyResponseKind::Found(self.0.clone())),
            }
        }

        async fn list_keys(
            &self,
            peer_id: Uuid,
        ) -> Result<Vec<PeerKey>, MappedErrors> {
            Ok(["1", "2"]
                .map(|kid| PeerKey {
                    peer_id,
                    kid: kid.to_string(),
                    alg: "EdDSA".to_string(),
                    public_key: "x".to_string(),
                    fetched_at: Local::now(),
                    expires_at: None,
                })
                .to_vec())
        }
    }

    fn peer() -> FederationPeer {
        FederationPeer {
            id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            base_url: "https://myc-02.example.com".to_string(),
            display_name: None,
            status: PeerStatus::Degraded,
            consume_kinds: vec![],
            serve_kinds: vec![],
            negotiated_kinds: vec!["profile.v1".to_string()],
            policy: FederationPolicy::default(),
            last_contact_at: Some(Local::now()),
            failure_count: 2,
            created: Local::now(),
            created_by: None,
            updated: None,
            updated_by: None,
        }
    }

    #[tokio::test]
    async fn each_peer_carries_its_status_kinds_keys_and_health(
    ) -> Result<(), MappedErrors> {
        let listed = Listed(vec![peer()]);

        let overviews =
            list_federation_peers(Box::new(&listed as &dyn FederationPeerRead))
                .await?;
        let json = serde_json::to_value(&overviews[0]).unwrap();

        assert_eq!(json["status"], "degraded");
        assert_eq!(json["negotiatedKinds"][0], "profile.v1");
        assert_eq!(json["keyIds"], serde_json::json!(["1", "2"]));
        assert_eq!(json["failureCount"], 2);
        assert!(json["lastContactAt"].is_string());

        Ok(())
    }

    #[tokio::test]
    async fn no_peers_is_an_empty_list() -> Result<(), MappedErrors> {
        let listed = Listed(vec![]);

        assert!(list_federation_peers(Box::new(
            &listed as &dyn FederationPeerRead
        ))
        .await?
        .is_empty());

        Ok(())
    }
}
