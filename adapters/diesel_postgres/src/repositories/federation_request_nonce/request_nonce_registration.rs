use crate::{
    models::config::DbPoolProvider,
    schema::federation_request_nonce as nonce_model,
};

use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use diesel::{dsl::now, prelude::*};
use myc_core::domain::entities::RequestNonceRegistration;
use mycelium_base::utils::errors::{creation_err, deletion_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RequestNonceRegistration)]
pub struct FederationRequestNonceSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl RequestNonceRegistration for FederationRequestNonceSqlDbRepository {
    /// The primary key on `jti` makes the claim atomic across every pod: the
    /// first insert wins, a replay inserts nothing
    #[tracing::instrument(name = "federation_request_nonce_claim", skip_all)]
    async fn claim(
        &self,
        jti: String,
        peer_id: Uuid,
        expires_at: DateTime<Local>,
    ) -> Result<bool, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
        })?;

        let inserted = diesel::insert_into(nonce_model::table)
            .values((
                nonce_model::jti.eq(&jti),
                nonce_model::peer_id.eq(peer_id),
                nonce_model::expires_at.eq(expires_at.with_timezone(&Utc)),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to claim request nonce: {e}"))
            })?;

        Ok(inserted == 1)
    }

    #[tracing::instrument(name = "federation_request_nonce_purge", skip_all)]
    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {e}"))
        })?;

        let deleted = diesel::delete(
            nonce_model::table.filter(nonce_model::expires_at.lt(now)),
        )
        .execute(conn)
        .map_err(|e| {
            deletion_err(format!("Failed to purge request nonces: {e}"))
        })?;

        Ok(deleted as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::DbPool;

    use chrono::Duration;
    use diesel::{
        connection::SimpleConnection,
        r2d2::{ConnectionManager, Pool},
        PgConnection,
    };
    use std::sync::Once;

    const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

    /// Own schema, so these tests never race the other live tests.
    const SCHEMA: &str = "myc_federation_nonce_test";

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

    fn pool() -> Option<DbPool> {
        let Ok(url) = std::env::var(DATABASE_URL_VAR) else {
            eprintln!(
                "skipping: set {DATABASE_URL_VAR} to run the live request \
                 nonce tests"
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

        Some(pool)
    }

    /// One repository per pool: two pools stand in for two pods.
    fn repository(pool: DbPool) -> FederationRequestNonceSqlDbRepository {
        FederationRequestNonceSqlDbRepository {
            db_config: Arc::new(TestPool(pool)),
        }
    }

    fn peer(pool: &DbPool) -> Uuid {
        let id = Uuid::new_v4();

        pool.get()
            .unwrap()
            .batch_execute(&format!(
                "INSERT INTO federation_peer (id, instance_id, base_url)
                 VALUES ('{id}', '{}', 'https://myc-02.example.com')",
                Uuid::new_v4()
            ))
            .unwrap();

        id
    }

    fn jti() -> String {
        Uuid::new_v4().to_string()
    }

    #[tokio::test]
    async fn a_replayed_jti_is_refused_from_another_pod() {
        let Some(first_pool) = pool() else {
            return;
        };
        let second_pool = pool().unwrap();
        let peer_id = peer(&first_pool);
        let first_pod = repository(first_pool);
        let second_pod = repository(second_pool);
        let jti = jti();
        let expires_at = Local::now() + Duration::minutes(1);

        assert!(first_pod
            .claim(jti.clone(), peer_id, expires_at)
            .await
            .unwrap());
        assert!(!second_pod
            .claim(jti.clone(), peer_id, expires_at)
            .await
            .unwrap());
        assert!(!first_pod.claim(jti, peer_id, expires_at).await.unwrap());
    }

    #[tokio::test]
    async fn purge_deletes_only_expired_nonces() {
        let Some(pool) = pool() else {
            return;
        };
        let peer_id = peer(&pool);
        let repo = repository(pool);
        let expired = jti();
        let live = jti();

        repo.claim(
            expired.clone(),
            peer_id,
            Local::now() - Duration::minutes(1),
        )
        .await
        .unwrap();
        repo.claim(live.clone(), peer_id, Local::now() + Duration::minutes(5))
            .await
            .unwrap();

        assert!(repo.purge_expired().await.unwrap() >= 1);

        // The expired jti can be claimed again; the live one is still held.
        assert!(repo
            .claim(expired, peer_id, Local::now() + Duration::minutes(1))
            .await
            .unwrap());
        assert!(!repo
            .claim(live, peer_id, Local::now() + Duration::minutes(5))
            .await
            .unwrap());
    }
}
