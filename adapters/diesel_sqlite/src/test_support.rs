#![cfg(test)]

//! Shared test scaffolding for SQLite repository integration tests: a fresh,
//! migrated temp database wrapped in a `SqliteDbPoolProvider`.

use super::config::{DieselSqliteDbPoolProvider, SqliteDbPoolProvider};
use super::migration::run_pending_migrations;

use diesel::{Connection, SqliteConnection};
use std::{path::PathBuf, sync::Arc};

pub(crate) struct TempDb {
    pub(crate) provider: Arc<dyn SqliteDbPoolProvider>,
    path: PathBuf,
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(self.path.with_extension("db-wal"));
        let _ = std::fs::remove_file(self.path.with_extension("db-shm"));
    }
}

pub(crate) fn setup_temp_db() -> TempDb {
    let path = std::env::temp_dir().join(format!(
        "myc_sqlite_repo_test_{}_{}.db",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let url = path.to_str().unwrap();

    let mut conn = SqliteConnection::establish(url).unwrap();
    run_pending_migrations(&mut conn).unwrap();
    drop(conn);

    let pool = DieselSqliteDbPoolProvider::new(url);

    TempDb {
        provider: Arc::new(DieselSqliteDbPoolProvider { pool }),
        path,
    }
}
