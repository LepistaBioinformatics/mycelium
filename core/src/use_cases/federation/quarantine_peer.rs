use crate::domain::{
    dtos::federation::{FederationPeer, PeerStatus},
    entities::{AssertionStoreWrite, FederationPeerRead, FederationPeerWrite},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Stop serving a peer (FED-34)
///
/// Effective on the peer's next request: the signed-request extractor reads
/// the status from the table on every call, with no cache to invalidate.
///
#[tracing::instrument(name = "quarantine_peer", skip_all)]
pub async fn quarantine_peer(
    instance_id: Uuid,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    peer_write_repo: Box<&dyn FederationPeerWrite>,
    assertion_store: Box<&dyn AssertionStoreWrite>,
) -> Result<FederationPeer, MappedErrors> {
    let peer = move_peer(
        instance_id,
        PeerStatus::Quarantined,
        *peer_read_repo,
        *peer_write_repo,
    )
    .await?;

    assertion_store.forget_peer(peer.id).await?;
    Ok(peer)
}

#[tracing::instrument(name = "release_peer", skip_all)]
pub async fn release_peer(
    instance_id: Uuid,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    peer_write_repo: Box<&dyn FederationPeerWrite>,
) -> Result<FederationPeer, MappedErrors> {
    move_peer(
        instance_id,
        PeerStatus::Declared,
        *peer_read_repo,
        *peer_write_repo,
    )
    .await
}

async fn move_peer(
    instance_id: Uuid,
    status: PeerStatus,
    peer_read_repo: &dyn FederationPeerRead,
    peer_write_repo: &dyn FederationPeerWrite,
) -> Result<FederationPeer, MappedErrors> {
    let FetchResponseKind::Found(peer) =
        peer_read_repo.get_by_instance_id(instance_id).await?
    else {
        return use_case_err(format!("no peer with instance id {instance_id}"))
            .with_exp_true()
            .as_error();
    };

    match peer_write_repo.update_status(peer.id, status).await? {
        UpdatingResponseKind::Updated(peer) => Ok(peer),
        UpdatingResponseKind::NotUpdated(_, reason) => {
            use_case_err(reason).as_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::{
            federation::{
                FederationPolicy, NewFederationPeer, NewPeerKey, PeerAdmission,
                PeerKey, ProfileAssertion, StoredAssertion, VerifiedAssertion,
            },
            written_by::WrittenBy,
        },
        use_cases::federation::{
            admit_requesting_peer, resolution_test_doubles::Mesh,
        },
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::{
        CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
    };
    use std::sync::Mutex;

    /// One stored peer whose status the use cases change
    struct Table(Mutex<FederationPeer>);

    #[async_trait]
    impl FederationPeerRead for Table {
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
            let peer = self.0.lock().unwrap().clone();

            match peer.instance_id == instance_id {
                true => Ok(FetchResponseKind::Found(peer)),
                false => Ok(FetchResponseKind::NotFound(Some(instance_id))),
            }
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
            unreachable!()
        }
    }

    #[async_trait]
    impl FederationPeerWrite for Table {
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
            let mut peer = self.0.lock().unwrap();
            peer.status = status;
            Ok(UpdatingResponseKind::Updated(peer.clone()))
        }

        async fn settle_status(
            &self,

            _: Uuid,

            _: PeerStatus,
        ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors>
        {
            unreachable!()
        }

        async fn record_keys(
            &self,
            _: Uuid,
            _: Vec<NewPeerKey>,
        ) -> Result<(), MappedErrors> {
            unreachable!()
        }

        async fn set_negotiated_kinds(
            &self,
            _: Uuid,
            _: Vec<String>,
        ) -> Result<(), MappedErrors> {
            unreachable!()
        }

        async fn record_success(&self, _: Uuid) -> Result<(), MappedErrors> {
            unreachable!()
        }

        async fn record_failure(&self, _: Uuid) -> Result<u32, MappedErrors> {
            unreachable!()
        }

        async fn remove(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unreachable!()
        }
    }

    fn table() -> Table {
        Table(Mutex::new(FederationPeer {
            id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            base_url: "https://myc-02.example.com".to_string(),
            display_name: None,
            status: PeerStatus::Active,
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
        }))
    }

    fn stored_from(peer_id: Uuid) -> StoredAssertion {
        let now = Local::now();
        StoredAssertion {
            link_id: Uuid::new_v4(),
            peer_id,
            assertion: VerifiedAssertion {
                jws: "jws".to_string(),
                payload: ProfileAssertion::default(),
                issued_at: now,
                expires_at: now + chrono::Duration::seconds(60),
            },
        }
    }

    fn instance_id(table: &Table) -> Uuid {
        table.0.lock().unwrap().instance_id
    }

    async fn admission(table: &Table) -> PeerAdmission {
        admit_requesting_peer(
            instance_id(table),
            Box::new(table as &dyn FederationPeerRead),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_quarantined_peer_is_refused_on_its_next_request(
    ) -> Result<(), MappedErrors> {
        let table = table();

        let mesh = Mesh::default();
        mesh.stored
            .lock()
            .unwrap()
            .push(stored_from(table.0.lock().unwrap().id));

        let peer = quarantine_peer(
            instance_id(&table),
            Box::new(&table as &dyn FederationPeerRead),
            Box::new(&table as &dyn FederationPeerWrite),
            Box::new(&mesh as &dyn AssertionStoreWrite),
        )
        .await?;

        assert_eq!(peer.status, PeerStatus::Quarantined);
        assert!(mesh.stored.lock().unwrap().is_empty());
        assert_eq!(
            admission(&table).await,
            PeerAdmission::Refused(PeerStatus::Quarantined)
        );

        Ok(())
    }

    #[tokio::test]
    async fn a_released_peer_is_declared_again_and_admitted(
    ) -> Result<(), MappedErrors> {
        let table = table();
        let read = Box::new(&table as &dyn FederationPeerRead);
        let write = Box::new(&table as &dyn FederationPeerWrite);

        let mesh = Mesh::default();
        let store = Box::new(&mesh as &dyn AssertionStoreWrite);
        quarantine_peer(
            instance_id(&table),
            read.clone(),
            write.clone(),
            store,
        )
        .await?;
        let peer = release_peer(instance_id(&table), read, write).await?;

        assert_eq!(peer.status, PeerStatus::Declared);
        assert!(matches!(
            admission(&table).await,
            PeerAdmission::Admitted(_)
        ));

        Ok(())
    }

    #[tokio::test]
    async fn an_unknown_peer_cannot_be_quarantined() {
        let table = table();

        let mesh = Mesh::default();
        let outcome = quarantine_peer(
            Uuid::new_v4(),
            Box::new(&table as &dyn FederationPeerRead),
            Box::new(&table as &dyn FederationPeerWrite),
            Box::new(&mesh as &dyn AssertionStoreWrite),
        )
        .await;

        assert!(outcome.is_err());
    }
}
