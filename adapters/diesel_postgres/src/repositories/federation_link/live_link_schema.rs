//! Live Postgres schema shared by the identity link tests

use crate::models::config::{DbPool, DbPoolProvider};

use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use std::sync::{Arc, Once};
use uuid::Uuid;

const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

/// Own schema, so these tests never race the other live tests.
const SCHEMA: &str = "myc_federation_link_test";

const TRUST: &str =
    include_str!("../../../sql/migrations/20261121_01_federation_trust.sql");

const LINKS: &str =
    include_str!("../../../sql/migrations/20261122_01_federation_links.sql");

static PREPARE: Once = Once::new();

struct TestPool(DbPool);

impl DbPoolProvider for TestPool {
    fn get_pool(&self) -> DbPool {
        self.0.clone()
    }
}

/// The migrations as written, with `public.user` swapped for a minimal table
/// of this schema (the foreign keys only need its `id`)
fn tables() -> String {
    let trust = TRUST.split("GRANT ALL").next().unwrap();
    let links = LINKS
        .split("GRANT ALL")
        .next()
        .unwrap()
        .replace("public.user", "link_test_user");

    format!(
        "DROP TABLE IF EXISTS federation_link_consent;
         DROP TABLE IF EXISTS federation_link_request;
         DROP TABLE IF EXISTS federation_identity_link;
         DROP TABLE IF EXISTS federation_request_nonce;
         DROP TABLE IF EXISTS federation_peer_key;
         DROP TABLE IF EXISTS federation_peer;
         DROP TABLE IF EXISTS link_test_user;
         CREATE TABLE link_test_user (id UUID PRIMARY KEY);
         {trust}
         {links}"
    )
}

/// The pool on the prepared schema, or `None` (after reporting the skip)
/// when no test database is configured
pub(crate) fn live_pool() -> Option<DbPool> {
    let Ok(url) = std::env::var(DATABASE_URL_VAR) else {
        eprintln!(
            "skipping: set {DATABASE_URL_VAR} to run the live identity link \
             tests"
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
        pool.get()
            .expect("failed to take a test connection")
            .batch_execute(&format!(
                "CREATE SCHEMA IF NOT EXISTS {SCHEMA}; {}",
                tables()
            ))
            .expect("failed to prepare the test schema");
    });

    Some(pool)
}

pub(crate) fn provider(pool: DbPool) -> Arc<dyn DbPoolProvider> {
    Arc::new(TestPool(pool))
}

/// A fresh user and a fresh peer
pub(crate) fn user_and_peer(pool: &DbPool) -> (Uuid, Uuid) {
    let (user_id, peer_id) = (Uuid::new_v4(), Uuid::new_v4());

    pool.get()
        .unwrap()
        .batch_execute(&format!(
            "INSERT INTO link_test_user (id) VALUES ('{user_id}');
             INSERT INTO federation_peer (id, instance_id, base_url)
             VALUES ('{peer_id}', gen_random_uuid(),
                     'https://myc-02.example.com');"
        ))
        .unwrap();

    (user_id, peer_id)
}
