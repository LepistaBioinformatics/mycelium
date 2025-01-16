use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use shaku::Interface;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub trait DbPoolProvider: Interface + Send + Sync {
    fn get_pool(&self) -> DbPool;
}
