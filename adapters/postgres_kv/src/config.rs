use crate::schema::kv_artifact;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use shaku::{Component, Interface};
use std::time::Duration;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub trait PgKvPoolProvider: Interface + Send + Sync {
    fn get_pool(&self) -> DbPool;
}

#[derive(Component)]
#[shaku(interface = PgKvPoolProvider)]
#[derive(Clone)]
pub struct PgKvPoolProviderImpl {
    pub pool: DbPool,
}

impl PgKvPoolProvider for PgKvPoolProviderImpl {
    fn get_pool(&self) -> DbPool {
        self.pool.clone()
    }
}

pub fn spawn_expiry_sweeper(pool: DbPool, interval_secs: u64) {
    tokio::spawn(run_expiry_sweeper(pool, interval_secs));
}

#[tracing::instrument(name = "run_expiry_sweeper", skip_all)]
async fn run_expiry_sweeper(pool: DbPool, interval_secs: u64) {
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        ticker.tick().await;
        sweep_expired_artifacts(&pool);
    }
}

fn sweep_expired_artifacts(pool: &DbPool) {
    let Ok(mut conn) = pool.get() else {
        tracing::error!("Failed to get DB connection for expiry sweep");
        return;
    };

    let deletion = diesel::delete(
        kv_artifact::table
            .filter(kv_artifact::expires_at.le(chrono::Utc::now())),
    )
    .execute(&mut conn);

    if let Err(e) = deletion {
        tracing::error!("Failed to sweep expired artifacts: {e}");
    }
}
