use crate::models::config::{DbPool, DbPoolProvider};

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = DbPoolProvider)]
#[derive(Debug, Clone)]
pub struct DieselDbPoolProvider {
    pool: DbPool,
}

impl DbPoolProvider for DieselDbPoolProvider {
    fn get_pool(&self) -> DbPool {
        self.pool.clone()
    }
}

impl DieselDbPoolProvider {
    pub fn new(database_url: &str) -> Pool<ConnectionManager<PgConnection>> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);

        match Pool::builder().build(manager) {
            Ok(pool) => pool,
            Err(e) => panic!("Failed to create database pool: {e}"),
        }
    }
}
