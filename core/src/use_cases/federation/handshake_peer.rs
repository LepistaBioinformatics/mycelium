use crate::domain::{
    dtos::federation::{
        FederationPeer, FederationPublicJwk, InstanceSelfDescription,
        NewPeerKey, PeerStatus,
    },
    entities::{FederationPeerWrite, PeerClient},
};

use mycelium_base::utils::errors::MappedErrors;

/// Handshake with a declared peer (FED-08)
///
/// Fetches the peer's self-description and checks it announces the
/// `instanceId` the operator declared: a different one marks the peer
/// untrusted and stores nothing, never re-pinning it. On a match the peer's
/// Ed25519 keys are stored, the assertion kinds both sides support are
/// recorded and the peer becomes active. An unreachable peer counts a
/// failure. Returns the status the peer ends in.
///
#[tracing::instrument(name = "handshake_peer", skip_all)]
pub async fn handshake_peer(
    peer: &FederationPeer,
    local_kinds: &[String],
    peer_client: Box<&dyn PeerClient>,
    peer_write_repo: Box<&dyn FederationPeerWrite>,
) -> Result<PeerStatus, MappedErrors> {
    let Ok(description) =
        peer_client.fetch_self_description(&peer.base_url).await
    else {
        peer_write_repo.record_failure(peer.id).await?;
        return settle(peer, PeerStatus::Unreachable, *peer_write_repo).await;
    };

    if description.instance_id != peer.instance_id {
        tracing::warn!(declared = %peer.instance_id, announced = %description.instance_id, "peer instance id mismatch");
        return settle(peer, PeerStatus::Untrusted, *peer_write_repo).await;
    }

    trust(peer, &description, local_kinds, *peer_write_repo).await
}

async fn trust(
    peer: &FederationPeer,
    description: &InstanceSelfDescription,
    local_kinds: &[String],
    peer_write_repo: &dyn FederationPeerWrite,
) -> Result<PeerStatus, MappedErrors> {
    let negotiated = local_kinds
        .iter()
        .filter(|kind| description.assertion_kinds.contains(kind))
        .cloned()
        .collect();

    peer_write_repo
        .record_keys(peer.id, usable_keys(&description.keys))
        .await?;
    peer_write_repo
        .set_negotiated_kinds(peer.id, negotiated)
        .await?;
    peer_write_repo.record_success(peer.id).await?;
    settle(peer, PeerStatus::Active, peer_write_repo).await
}

async fn settle(
    peer: &FederationPeer,
    status: PeerStatus,
    peer_write_repo: &dyn FederationPeerWrite,
) -> Result<PeerStatus, MappedErrors> {
    peer_write_repo.update_status(peer.id, status).await?;
    Ok(status)
}

/// The peer's Ed25519 signing keys; anything else is ignored
fn usable_keys(keys: &[FederationPublicJwk]) -> Vec<NewPeerKey> {
    keys.iter()
        .filter(|key| {
            key.kty == "OKP" && key.crv == "Ed25519" && key.alg == "EdDSA"
        })
        .map(|key| NewPeerKey {
            kid: key.kid.clone(),
            alg: key.alg.clone(),
            public_key: key.x.clone(),
            expires_at: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        federation::{FederationPolicy, NewFederationPeer},
        written_by::WrittenBy,
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::{
        entities::{
            CreateResponseKind, DeletionResponseKind, UpdatingResponseKind,
        },
        utils::errors::execution_err,
    };
    use std::sync::Mutex;
    use uuid::Uuid;

    struct Peer(Option<InstanceSelfDescription>);

    #[async_trait]
    impl PeerClient for Peer {
        async fn fetch_self_description(
            &self,
            _: &str,
        ) -> Result<InstanceSelfDescription, MappedErrors> {
            self.0
                .clone()
                .ok_or_else(|| execution_err("peer unreachable"))
        }

        async fn post_signed(
            &self,
            _: &str,
            _: &str,
            _: Vec<u8>,
            _: &str,
        ) -> Result<Vec<u8>, MappedErrors> {
            unreachable!()
        }
    }

    #[derive(Default)]
    struct Recorded {
        status: Mutex<Vec<PeerStatus>>,
        keys: Mutex<Option<Vec<NewPeerKey>>>,
        kinds: Mutex<Option<Vec<String>>>,
        failures: Mutex<u32>,
        successes: Mutex<u32>,
    }

    #[async_trait]
    impl FederationPeerWrite for Recorded {
        async fn declare(
            &self,
            _: NewFederationPeer,
            _: Option<WrittenBy>,
        ) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
            unreachable!()
        }

        async fn update_status(
            &self,
            _: Uuid,
            status: PeerStatus,
        ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors>
        {
            self.status.lock().unwrap().push(status);
            Ok(UpdatingResponseKind::Updated(declared()))
        }

        async fn record_keys(
            &self,
            _: Uuid,
            keys: Vec<NewPeerKey>,
        ) -> Result<(), MappedErrors> {
            *self.keys.lock().unwrap() = Some(keys);
            Ok(())
        }

        async fn set_negotiated_kinds(
            &self,
            _: Uuid,
            kinds: Vec<String>,
        ) -> Result<(), MappedErrors> {
            *self.kinds.lock().unwrap() = Some(kinds);
            Ok(())
        }

        async fn record_success(&self, _: Uuid) -> Result<(), MappedErrors> {
            *self.successes.lock().unwrap() += 1;
            Ok(())
        }

        async fn record_failure(&self, _: Uuid) -> Result<u32, MappedErrors> {
            let mut failures = self.failures.lock().unwrap();
            *failures += 1;
            Ok(*failures)
        }

        async fn remove(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unreachable!()
        }
    }

    fn declared() -> FederationPeer {
        FederationPeer {
            id: Uuid::from_u128(1),
            instance_id: Uuid::from_u128(2),
            base_url: "https://myc-02.example.com".to_string(),
            display_name: None,
            status: PeerStatus::Declared,
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

    fn jwk(kid: &str, kty: &str) -> FederationPublicJwk {
        FederationPublicJwk {
            kty: kty.to_string(),
            crv: "Ed25519".to_string(),
            x: format!("x-{kid}"),
            kid: kid.to_string(),
            key_use: "sig".to_string(),
            alg: "EdDSA".to_string(),
        }
    }

    fn description(instance_id: Uuid) -> InstanceSelfDescription {
        InstanceSelfDescription {
            instance_id,
            display_name: None,
            base_url: "https://myc-02.example.com".to_string(),
            version: "9.0.1".to_string(),
            assertion_kinds: vec!["profile.v1".to_string(), "x.v1".to_string()],
            keys: vec![jwk("1", "OKP"), jwk("2", "OKP"), jwk("3", "RSA")],
            role_catalog_url: None,
        }
    }

    async fn run(
        peer: &Peer,
        recorded: &Recorded,
    ) -> Result<PeerStatus, MappedErrors> {
        handshake_peer(
            &declared(),
            &["profile.v1".to_string()],
            Box::new(peer as &dyn PeerClient),
            Box::new(recorded as &dyn FederationPeerWrite),
        )
        .await
    }

    #[tokio::test]
    async fn a_matching_peer_becomes_active_with_its_keys(
    ) -> Result<(), MappedErrors> {
        let recorded = Recorded::default();

        let status =
            run(&Peer(Some(description(declared().instance_id))), &recorded)
                .await?;

        assert_eq!(status, PeerStatus::Active);
        assert_eq!(*recorded.status.lock().unwrap(), [PeerStatus::Active]);
        let kids: Vec<_> = recorded
            .keys
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .into_iter()
            .map(|key| key.kid)
            .collect();
        assert_eq!(kids, ["1", "2"]);
        assert_eq!(
            recorded.kinds.lock().unwrap().clone().unwrap(),
            ["profile.v1"]
        );
        assert_eq!(*recorded.successes.lock().unwrap(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn a_mismatched_instance_id_is_untrusted_and_stores_nothing(
    ) -> Result<(), MappedErrors> {
        let recorded = Recorded::default();

        let status =
            run(&Peer(Some(description(Uuid::from_u128(99)))), &recorded)
                .await?;

        assert_eq!(status, PeerStatus::Untrusted);
        assert_eq!(*recorded.status.lock().unwrap(), [PeerStatus::Untrusted]);
        assert!(recorded.keys.lock().unwrap().is_none());
        assert_eq!(*recorded.successes.lock().unwrap(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn an_unreachable_peer_counts_a_failure() -> Result<(), MappedErrors>
    {
        let recorded = Recorded::default();

        let status = run(&Peer(None), &recorded).await?;

        assert_eq!(status, PeerStatus::Unreachable);
        assert_eq!(*recorded.failures.lock().unwrap(), 1);
        assert!(recorded.keys.lock().unwrap().is_none());

        Ok(())
    }
}
