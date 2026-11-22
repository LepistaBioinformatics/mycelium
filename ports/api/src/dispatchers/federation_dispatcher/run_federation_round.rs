use super::FederationRoundPorts;

use chrono::{DateTime, Local};
use myc_core::{
    domain::dtos::federation::FederationPeer,
    use_cases::federation::{
        handshake_peer, is_federation_enabled, peer_may_be_called,
    },
};
use mycelium_base::entities::FetchManyResponseKind;
use std::time::Duration;

/// How long an open circuit waits before one probe
pub(crate) const PEER_COOL_DOWN: Duration = Duration::from_secs(60);

/// One round of the federation dispatcher
///
/// Skipped while the runtime switch is off (kill-switch, FED-35). Otherwise:
/// purge expired request nonces, then handshake again with every peer the
/// circuit lets through, which refreshes its keys and moves its status
/// (FED-20, FED-21). A failure is logged and the round goes on; nothing here
/// panics.
///
#[tracing::instrument(name = "run_federation_round", skip_all)]
pub(crate) async fn run_federation_round(
    ports: &FederationRoundPorts,
    local_kinds: &[String],
    max_failures_before_open: u32,
    now: DateTime<Local>,
) {
    if !matches!(
        is_federation_enabled(Box::new(ports.settings.as_ref())).await,
        Ok(true)
    ) {
        return;
    }

    if let Err(err) = ports.nonces.purge_expired().await {
        tracing::error!(%err, "request nonce purge failed");
    }

    for peer in callable_peers(ports, now).await {
        handshake_logged(ports, &peer, local_kinds, max_failures_before_open)
            .await;
    }
}

async fn handshake_logged(
    ports: &FederationRoundPorts,
    peer: &FederationPeer,
    local_kinds: &[String],
    max_failures_before_open: u32,
) {
    let outcome = handshake_peer(
        peer,
        local_kinds,
        max_failures_before_open,
        Box::new(ports.peer_client.as_ref()),
        Box::new(ports.peer_write.as_ref()),
    )
    .await;

    if let Err(err) = outcome {
        tracing::error!(%err, peer = %peer.instance_id, "peer health round failed");
    }
}

async fn callable_peers(
    ports: &FederationRoundPorts,
    now: DateTime<Local>,
) -> Vec<FederationPeer> {
    let peers = match ports.peer_read.list().await {
        Ok(FetchManyResponseKind::Found(peers)) => peers,
        Ok(_) => return vec![],
        Err(err) => {
            tracing::error!(%err, "peer list unreadable");
            return vec![];
        }
    };

    peers
        .into_iter()
        .filter(|peer| {
            let changed = peer.updated.unwrap_or(peer.created);
            let since = (now - changed).to_std().unwrap_or_default();

            peer_may_be_called(peer.status, since, PEER_COOL_DOWN)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use chrono::Duration as ChronoDuration;
    use myc_core::domain::{
        dtos::{
            federation::{
                FederationPolicy, InstanceSelfDescription, NewFederationPeer,
                NewPeerKey, PeerKey, PeerStatus,
            },
            instance_settings::InstanceSetting,
            written_by::WrittenBy,
        },
        entities::{
            FederationPeerRead, FederationPeerWrite, InstanceSettingsFetching,
            PeerClient, RequestNonceRegistration,
        },
    };
    use mycelium_base::{
        entities::{
            CreateResponseKind, DeletionResponseKind, FetchResponseKind,
            UpdatingResponseKind,
        },
        utils::errors::{execution_err, MappedErrors},
    };
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// Every port of a round in one double: the peers to list, the base URLs
    /// that were called, the statuses written and the purges run
    #[derive(Default)]
    struct Mesh {
        enabled: bool,
        peers: Vec<FederationPeer>,
        called: Mutex<Vec<String>>,
        statuses: Mutex<Vec<(Uuid, PeerStatus)>>,
        purges: Mutex<u32>,
    }

    #[async_trait]
    impl InstanceSettingsFetching for Mesh {
        async fn get(
            &self,
            key: String,
        ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors>
        {
            Ok(FetchResponseKind::Found(InstanceSetting {
                key,
                value: serde_json::json!(self.enabled),
                created_by: None,
                updated_by: None,
                created: Local::now(),
                updated: None,
            }))
        }
    }

    #[async_trait]
    impl RequestNonceRegistration for Mesh {
        async fn claim(
            &self,
            _: String,
            _: Uuid,
            _: DateTime<Local>,
        ) -> Result<bool, MappedErrors> {
            unreachable!()
        }

        async fn purge_expired(&self) -> Result<u64, MappedErrors> {
            *self.purges.lock().unwrap() += 1;
            Err(execution_err("a failing purge must not stop the round"))
        }
    }

    #[async_trait]
    impl FederationPeerRead for Mesh {
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
            Ok(FetchManyResponseKind::Found(self.peers.clone()))
        }

        async fn list_keys(
            &self,
            _: Uuid,
        ) -> Result<Vec<PeerKey>, MappedErrors> {
            unreachable!()
        }
    }

    #[async_trait]
    impl FederationPeerWrite for Mesh {
        async fn declare(
            &self,
            _: NewFederationPeer,
            _: Option<WrittenBy>,
        ) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
            unreachable!()
        }

        async fn update_status(
            &self,
            peer_id: Uuid,
            status: PeerStatus,
        ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors>
        {
            self.statuses.lock().unwrap().push((peer_id, status));
            Ok(UpdatingResponseKind::Updated(FederationPeer {
                status,
                ..self.peers[0].clone()
            }))
        }

        async fn settle_status(
            &self,

            peer_id: Uuid,

            status: PeerStatus,
        ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors>
        {
            self.update_status(peer_id, status).await
        }

        async fn record_keys(
            &self,
            _: Uuid,
            _: Vec<NewPeerKey>,
        ) -> Result<(), MappedErrors> {
            Ok(())
        }

        async fn set_negotiated_kinds(
            &self,
            _: Uuid,
            _: Vec<String>,
        ) -> Result<(), MappedErrors> {
            Ok(())
        }

        async fn record_success(&self, _: Uuid) -> Result<(), MappedErrors> {
            Ok(())
        }

        async fn record_failure(&self, _: Uuid) -> Result<u32, MappedErrors> {
            Ok(1)
        }

        async fn remove(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unreachable!()
        }
    }

    #[async_trait]
    impl PeerClient for Mesh {
        async fn fetch_self_description(
            &self,
            base_url: &str,
        ) -> Result<InstanceSelfDescription, MappedErrors> {
            self.called.lock().unwrap().push(base_url.to_string());
            let peer = self
                .peers
                .iter()
                .find(|peer| peer.base_url == base_url)
                .unwrap();

            Ok(InstanceSelfDescription {
                instance_id: peer.instance_id,
                display_name: None,
                base_url: base_url.to_string(),
                version: "9.0.1".to_string(),
                assertion_kinds: vec![],
                keys: vec![],
                role_catalog_url: None,
            })
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

    fn peer(
        name: &str,
        status: PeerStatus,
        changed_ago: i64,
    ) -> FederationPeer {
        FederationPeer {
            id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            base_url: format!("https://{name}.example.com"),
            display_name: None,
            status,
            consume_kinds: vec![],
            serve_kinds: vec![],
            negotiated_kinds: vec![],
            policy: FederationPolicy::default(),
            last_contact_at: None,
            failure_count: 0,
            created: Local::now() - ChronoDuration::hours(1),
            created_by: None,
            updated: Some(Local::now() - ChronoDuration::seconds(changed_ago)),
            updated_by: None,
        }
    }

    fn ports(mesh: Arc<Mesh>) -> FederationRoundPorts {
        FederationRoundPorts {
            peer_read: mesh.clone(),
            peer_write: mesh.clone(),
            nonces: mesh.clone(),
            settings: mesh.clone(),
            peer_client: mesh,
        }
    }

    #[tokio::test]
    async fn a_round_purges_and_handshakes_only_callable_peers() {
        let mesh = Arc::new(Mesh {
            enabled: true,
            peers: vec![
                peer("active", PeerStatus::Active, 5),
                peer("quarantined", PeerStatus::Quarantined, 3600),
                peer("open", PeerStatus::Unreachable, 10),
                peer("probe", PeerStatus::Unreachable, 120),
            ],
            ..Mesh::default()
        });

        run_federation_round(&ports(mesh.clone()), &[], 5, Local::now()).await;

        assert_eq!(*mesh.purges.lock().unwrap(), 1);
        assert_eq!(
            *mesh.called.lock().unwrap(),
            ["https://active.example.com", "https://probe.example.com"]
        );
        assert!(mesh
            .statuses
            .lock()
            .unwrap()
            .iter()
            .all(|(_, status)| *status == PeerStatus::Active));
    }

    #[tokio::test]
    async fn nothing_runs_while_federation_is_switched_off() {
        let mesh = Arc::new(Mesh {
            enabled: false,
            peers: vec![peer("active", PeerStatus::Active, 5)],
            ..Mesh::default()
        });

        run_federation_round(&ports(mesh.clone()), &[], 5, Local::now()).await;

        assert_eq!(*mesh.purges.lock().unwrap(), 0);
        assert!(mesh.called.lock().unwrap().is_empty());
    }
}
