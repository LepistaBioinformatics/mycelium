use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::PgConnection;
use shaku::{Component, Interface};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Component)]
#[shaku(interface = DbConfig)]
pub struct PostgresConfig {
    connection_pool: PgPool,
}

pub trait DbConfig: Interface + Send + Sync {
    fn get_pool(&self) -> &PgPool;
}

impl DbConfig for PostgresConfig {
    fn get_pool(&self) -> &PgPool {
        &self.connection_pool
    }
}

impl PostgresConfig {
    pub fn new(database_url: &str) -> Result<Self, PoolError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().build(manager)?;

        Ok(PostgresConfig {
            connection_pool: pool,
        })
    }
}
