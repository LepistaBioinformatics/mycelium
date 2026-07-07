use diesel::connection::SimpleConnection;
use diesel::r2d2::{
    ConnectionManager, CustomizeConnection, Error as R2d2Error, Pool,
};
use diesel::SqliteConnection;
use shaku::{Component, Interface};

pub type SqliteDbPool = Pool<ConnectionManager<SqliteConnection>>;

pub trait SqliteDbPoolProvider: Interface + Send + Sync {
    fn get_pool(&self) -> SqliteDbPool;
}

#[derive(Component)]
#[shaku(interface = SqliteDbPoolProvider)]
#[derive(Debug, Clone)]
pub struct DieselSqliteDbPoolProvider {
    pub(crate) pool: SqliteDbPool,
}

impl SqliteDbPoolProvider for DieselSqliteDbPoolProvider {
    fn get_pool(&self) -> SqliteDbPool {
        self.pool.clone()
    }
}

// ? ----------------------------------------------------------------------------
// ? Connection pragmas
// ?
// ? WAL improves single-writer/multi-reader concurrency; foreign_keys enforces
// ? referential integrity (off by default in SQLite); busy_timeout avoids
// ? immediate `SQLITE_BUSY` under contention.
// ? ----------------------------------------------------------------------------

#[derive(Debug)]
struct SqlitePragmas;

impl CustomizeConnection<SqliteConnection, R2d2Error> for SqlitePragmas {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), R2d2Error> {
        conn.batch_execute(
            "PRAGMA journal_mode = WAL; \
             PRAGMA foreign_keys = ON; \
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(R2d2Error::QueryError)
    }
}

impl DieselSqliteDbPoolProvider {
    pub fn new(database_url: &str) -> SqliteDbPool {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);

        match Pool::builder()
            .connection_customizer(Box::new(SqlitePragmas))
            .build(manager)
        {
            Ok(pool) => pool,
            Err(e) => panic!("Failed to create SQLite database pool: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_types::Text;
    use diesel::{QueryableByName, RunQueryDsl};

    #[derive(QueryableByName)]
    struct PragmaValue {
        #[diesel(sql_type = Text)]
        journal_mode: String,
    }

    #[test]
    fn pragmas_are_applied_on_acquire() {
        let path = std::env::temp_dir()
            .join(format!("myc_sqlite_pragma_{}.db", std::process::id()));
        let url = path.to_str().unwrap();

        let pool = DieselSqliteDbPoolProvider::new(url);
        let mut conn = pool.get().unwrap();

        let journal: PragmaValue = diesel::sql_query("PRAGMA journal_mode")
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(journal.journal_mode.to_lowercase(), "wal");

        drop(conn);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }
}
