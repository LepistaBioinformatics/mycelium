use super::{
    change_peer_status::change_peer_status,
    declare_peer::declare_peer,
    record_peer_contact::{
        record_contact_failure, record_contact_success, record_negotiated_kinds,
    },
    replace_peer_keys::replace_peer_keys,
    settle_peer_status::settle_peer_status,
};
use crate::{
    models::config::DbPoolProvider, schema::federation_peer as peer_model,
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::{
        federation::{
            FederationPeer, NewFederationPeer, NewPeerKey, PeerStatus,
        },
        written_by::WrittenBy,
    },
    entities::FederationPeerWrite,
};
use mycelium_base::{
    entities::{
        CreateResponseKind, DeletionResponseKind, UpdatingResponseKind,
    },
    utils::errors::{deletion_err, updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = FederationPeerWrite)]
pub struct FederationPeerWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl FederationPeerWriteSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

#[async_trait]
impl FederationPeerWrite for FederationPeerWriteSqlDbRepository {
    #[tracing::instrument(name = "federation_peer_declare", skip_all)]
    async fn declare(
        &self,
        peer: NewFederationPeer,
        created_by: Option<WrittenBy>,
    ) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
        declare_peer(&mut *self.connection()?, peer, created_by)
    }

    #[tracing::instrument(name = "federation_peer_update_status", skip_all)]
    async fn update_status(
        &self,
        peer_id: Uuid,
        status: PeerStatus,
    ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
        change_peer_status(&mut *self.connection()?, peer_id, status)
    }

    #[tracing::instrument(name = "federation_peer_settle_status", skip_all)]
    async fn settle_status(
        &self,
        peer_id: Uuid,
        status: PeerStatus,
    ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
        settle_peer_status(&mut *self.connection()?, peer_id, status)
    }

    #[tracing::instrument(name = "federation_peer_record_keys", skip_all)]
    async fn record_keys(
        &self,
        peer_id: Uuid,
        keys: Vec<NewPeerKey>,
    ) -> Result<(), MappedErrors> {
        replace_peer_keys(&mut *self.connection()?, peer_id, &keys)
    }

    #[tracing::instrument(
        name = "federation_peer_set_negotiated_kinds",
        skip_all
    )]
    async fn set_negotiated_kinds(
        &self,
        peer_id: Uuid,
        kinds: Vec<String>,
    ) -> Result<(), MappedErrors> {
        record_negotiated_kinds(&mut *self.connection()?, peer_id, &kinds)
    }

    #[tracing::instrument(name = "federation_peer_record_success", skip_all)]
    async fn record_success(&self, peer_id: Uuid) -> Result<(), MappedErrors> {
        record_contact_success(&mut *self.connection()?, peer_id)
    }

    #[tracing::instrument(name = "federation_peer_record_failure", skip_all)]
    async fn record_failure(&self, peer_id: Uuid) -> Result<u32, MappedErrors> {
        record_contact_failure(&mut *self.connection()?, peer_id)
    }

    /// Keys and nonces go with the peer (ON DELETE CASCADE)
    #[tracing::instrument(name = "federation_peer_remove", skip_all)]
    async fn remove(
        &self,
        peer_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let deleted = diesel::delete(peer_model::table.find(peer_id))
            .execute(&mut *self.connection()?)
            .map_err(|e| deletion_err(format!("Failed to remove peer: {e}")))?;

        match deleted {
            0 => Ok(DeletionResponseKind::NotDeleted(
                peer_id,
                format!("peer {peer_id} not found"),
            )),
            _ => Ok(DeletionResponseKind::Deleted),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::{DbPool, DbPoolProvider};
    use crate::repositories::federation_peer::FederationPeerReadSqlDbRepository;

    use diesel::{
        connection::SimpleConnection,
        r2d2::{ConnectionManager, Pool},
        PgConnection,
    };
    use myc_core::domain::{
        dtos::federation::{
            FederationPolicy, NewFederationPeer, NewPeerKey, PeerStatus,
        },
        entities::{FederationPeerRead, FederationPeerWrite},
    };
    use mycelium_base::entities::{
        CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
        FetchResponseKind, UpdatingResponseKind,
    };
    use std::sync::{Arc, Once};
    use uuid::Uuid;

    /// Same variable as the other live tests of this crate. Without it every
    /// test below reports the skip and passes.
    const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

    /// Own schema, so these tests never race the other live tests.
    const SCHEMA: &str = "myc_federation_peer_test";

    /// The tables exactly as the migration creates them.
    const MIGRATION: &str = include_str!(
        "../../../sql/migrations/20261121_01_federation_trust.sql"
    );

    static PREPARE: Once = Once::new();

    struct TestPool(DbPool);

    impl DbPoolProvider for TestPool {
        fn get_pool(&self) -> DbPool {
            self.0.clone()
        }
    }

    fn live() -> Option<(
        FederationPeerReadSqlDbRepository,
        FederationPeerWriteSqlDbRepository,
    )> {
        let Ok(url) = std::env::var(DATABASE_URL_VAR) else {
            eprintln!(
                "skipping: set {DATABASE_URL_VAR} to run the live federation \
                 peer tests"
            );
            return None;
        };

        let separator = if url.contains('?') { '&' } else { '?' };
        let url = format!("{url}{separator}options=-csearch_path%3D{SCHEMA}");
        let pool = Pool::builder()
            .max_size(4)
            .build(ConnectionManager::<PgConnection>::new(url))
            .expect("failed to build the test pool");

        PREPARE.call_once(|| {
            let tables = MIGRATION.split("GRANT ALL").next().unwrap();

            pool.get()
                .expect("failed to take a test connection")
                .batch_execute(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {SCHEMA};
                     DROP TABLE IF EXISTS federation_request_nonce;
                     DROP TABLE IF EXISTS federation_peer_key;
                     DROP TABLE IF EXISTS federation_peer;
                     {tables}"
                ))
                .expect("failed to prepare the test schema");
        });

        let provider: Arc<dyn DbPoolProvider> = Arc::new(TestPool(pool));

        Some((
            FederationPeerReadSqlDbRepository {
                db_config: provider.clone(),
            },
            FederationPeerWriteSqlDbRepository {
                db_config: provider,
            },
        ))
    }

    fn new_peer() -> NewFederationPeer {
        NewFederationPeer {
            instance_id: Uuid::new_v4(),
            base_url: "https://myc-02.example.com".to_string(),
            display_name: Some("Instance 02".to_string()),
            consume_kinds: vec!["profile.v1".to_string()],
            serve_kinds: vec!["profile.v1".to_string()],
            policy: FederationPolicy::default(),
        }
    }

    fn key(kid: &str) -> NewPeerKey {
        NewPeerKey {
            kid: kid.to_string(),
            alg: "EdDSA".to_string(),
            public_key: format!("public-key-{kid}"),
            expires_at: None,
        }
    }

    async fn declared(write: &FederationPeerWriteSqlDbRepository) -> Uuid {
        match write.declare(new_peer(), None).await.unwrap() {
            CreateResponseKind::Created(peer) => peer.id,
            CreateResponseKind::NotCreated(_, reason) => panic!("{reason}"),
        }
    }

    #[tokio::test]
    async fn declare_then_read_by_id_and_instance_id() {
        let Some((read, write)) = live() else {
            return;
        };
        let new = new_peer();

        let CreateResponseKind::Created(peer) =
            write.declare(new.clone(), None).await.unwrap()
        else {
            panic!("first declaration must create");
        };

        assert_eq!(peer.status, PeerStatus::Declared);
        assert_eq!(peer.instance_id, new.instance_id);
        assert_eq!(peer.consume_kinds, new.consume_kinds);
        assert_eq!(peer.policy, FederationPolicy::default());
        assert_eq!(peer.failure_count, 0);

        let FetchResponseKind::Found(by_id) = read.get(peer.id).await.unwrap()
        else {
            panic!("declared peer must be found by id");
        };
        let FetchResponseKind::Found(by_instance) =
            read.get_by_instance_id(new.instance_id).await.unwrap()
        else {
            panic!("declared peer must be found by instance id");
        };

        assert_eq!(by_id, peer);
        assert_eq!(by_instance, peer);

        let FetchManyResponseKind::Found(all) = read.list().await.unwrap()
        else {
            panic!("list must find the declared peer");
        };
        assert!(all.iter().any(|one| one.id == peer.id));
    }

    #[tokio::test]
    async fn declaring_the_same_instance_id_twice_is_a_conflict() {
        let Some((read, write)) = live() else {
            return;
        };
        let new = new_peer();

        write.declare(new.clone(), None).await.unwrap();
        let second = write.declare(new.clone(), None).await.unwrap();

        assert!(matches!(second, CreateResponseKind::NotCreated(..)));

        let FetchManyResponseKind::Found(all) = read.list().await.unwrap()
        else {
            panic!("list must find the peer");
        };
        assert_eq!(
            all.iter()
                .filter(|one| one.instance_id == new.instance_id)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn record_keys_replaces_the_stored_set() {
        let Some((read, write)) = live() else {
            return;
        };
        let peer_id = declared(&write).await;

        write
            .record_keys(peer_id, vec![key("1"), key("2")])
            .await
            .unwrap();
        write
            .record_keys(peer_id, vec![key("2"), key("3")])
            .await
            .unwrap();

        let kids: Vec<_> = read
            .list_keys(peer_id)
            .await
            .unwrap()
            .into_iter()
            .map(|key| key.kid)
            .collect();

        assert_eq!(kids, ["2", "3"]);
    }

    #[tokio::test]
    async fn status_failures_and_success_are_recorded() {
        let Some((read, write)) = live() else {
            return;
        };
        let peer_id = declared(&write).await;

        let UpdatingResponseKind::Updated(peer) = write
            .update_status(peer_id, PeerStatus::Quarantined)
            .await
            .unwrap()
        else {
            panic!("status update must apply");
        };
        assert_eq!(peer.status, PeerStatus::Quarantined);

        assert_eq!(write.record_failure(peer_id).await.unwrap(), 1);
        assert_eq!(write.record_failure(peer_id).await.unwrap(), 2);

        write.record_success(peer_id).await.unwrap();
        write
            .set_negotiated_kinds(peer_id, vec!["profile.v1".to_string()])
            .await
            .unwrap();

        let FetchResponseKind::Found(peer) = read.get(peer_id).await.unwrap()
        else {
            panic!("peer must still exist");
        };
        assert_eq!(peer.failure_count, 0);
        assert!(peer.last_contact_at.is_some());
        assert_eq!(peer.negotiated_kinds, ["profile.v1"]);
    }

    #[tokio::test]
    async fn remove_purges_the_peer_and_its_keys() {
        let Some((read, write)) = live() else {
            return;
        };
        let peer_id = declared(&write).await;
        write.record_keys(peer_id, vec![key("1")]).await.unwrap();

        assert!(matches!(
            write.remove(peer_id).await.unwrap(),
            DeletionResponseKind::Deleted
        ));
        assert!(matches!(
            read.get(peer_id).await.unwrap(),
            FetchResponseKind::NotFound(_)
        ));
        assert!(read.list_keys(peer_id).await.unwrap().is_empty());
        assert!(matches!(
            write.remove(peer_id).await.unwrap(),
            DeletionResponseKind::NotDeleted(..)
        ));
    }

    #[tokio::test]
    async fn settling_never_lifts_a_quarantine() {
        let Some((read, write)) = live() else {
            return;
        };
        let peer_id = declared(&write).await;

        write
            .update_status(peer_id, PeerStatus::Quarantined)
            .await
            .unwrap();
        let settled = write
            .settle_status(peer_id, PeerStatus::Active)
            .await
            .unwrap();

        assert!(matches!(
            settled,
            UpdatingResponseKind::NotUpdated(ref peer, _)
                if peer.status == PeerStatus::Quarantined
        ));
        let FetchResponseKind::Found(peer) = read.get(peer_id).await.unwrap()
        else {
            panic!("peer must still exist");
        };
        assert_eq!(peer.status, PeerStatus::Quarantined);

        write
            .update_status(peer_id, PeerStatus::Declared)
            .await
            .unwrap();
        assert!(matches!(
            write.settle_status(peer_id, PeerStatus::Active).await.unwrap(),
            UpdatingResponseKind::Updated(ref peer)
                if peer.status == PeerStatus::Active
        ));
    }
}
