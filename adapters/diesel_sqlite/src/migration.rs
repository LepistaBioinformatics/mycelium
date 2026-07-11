use diesel::{connection::SimpleConnection, Connection, SqliteConnection};
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, MigrationHarness,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use std::path::Path;

/// Migrations embedded into the binary, so a standalone build auto-provisions
/// the SQLite database on first boot with no external migration step.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Run all pending embedded migrations against the given connection.
pub fn run_pending_migrations(
    conn: &mut SqliteConnection,
) -> Result<(), MappedErrors> {
    conn.run_pending_migrations(MIGRATIONS).map_err(|err| {
        creation_err(format!("Failed to run SQLite migrations: {err}"))
    })?;

    Ok(())
}

/// Auto-provisions the SQLite database at `url` on boot: creates the parent
/// directory if missing, establishes a one-off connection, and runs every
/// pending embedded migration. Callers (e.g. `ports/api`'s standalone
/// `initialize_modules`) build the actual connection pool afterward via
/// `DieselSqliteDbPoolProvider::new` -- this only needs to run once, before
/// the pool exists.
pub fn provision_database(url: &str) -> Result<(), MappedErrors> {
    if let Some(parent) = Path::new(url).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| {
                creation_err(format!(
                    "Failed to create SQLite database directory {parent:?}: {err}"
                ))
            })?;
        }
    }

    let mut conn = SqliteConnection::establish(url).map_err(|err| {
        creation_err(format!(
            "Failed to establish SQLite connection at {url}: {err}"
        ))
    })?;

    // This connection bypasses `DieselSqliteDbPoolProvider`'s pool
    // customizer (`SqlitePragmas`), so `busy_timeout` defaults to 0 --
    // `SQLITE_BUSY` ("database is locked") fails instantly with no retry.
    // That matters here specifically: this runs on every boot, and a
    // previous instance (container restart, Ctrl+C) can still be mid
    // shutdown and briefly holding the file. Match the pool's timeout so
    // this one-off connection retries too instead of failing the first
    // time it races a still-exiting previous process.
    conn.batch_execute("PRAGMA busy_timeout = 5000;")
        .map_err(|err| {
            creation_err(format!("Failed to set busy_timeout: {err}"))
        })?;

    run_pending_migrations(&mut conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_types::Text;
    use diesel::{QueryableByName, RunQueryDsl};

    #[derive(QueryableByName)]
    struct TableName {
        #[diesel(sql_type = Text)]
        name: String,
    }

    #[test]
    fn migrations_create_all_tables() {
        let path = std::env::temp_dir()
            .join(format!("myc_sqlite_migr_{}.db", std::process::id()));
        let url = path.to_str().unwrap();

        let mut conn = SqliteConnection::establish(url).unwrap();
        run_pending_migrations(&mut conn).unwrap();

        let rows: Vec<TableName> = diesel::sql_query(
            "SELECT name FROM sqlite_master WHERE type = 'table'",
        )
        .load(&mut conn)
        .unwrap();

        let names: Vec<String> = rows.into_iter().map(|r| r.name).collect();

        for expected in [
            "account",
            "account_tag",
            "error_code",
            "guest_role",
            "guest_role_children",
            "guest_user",
            "guest_user_on_account",
            "healthcheck_logs",
            "identity_provider",
            "manager_account_on_tenant",
            "owner_on_tenant",
            "tenant",
            "tenant_tag",
            "token",
            "user",
            "webhook",
            "webhook_execution",
            "message_queue",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "missing table: {expected}"
            );
        }

        let _ = std::fs::remove_file(&path);
    }

    /// Reproduces the real failure: `provision_database` runs on every boot,
    /// including a restart that races a still-shutting-down previous
    /// process. Holds an exclusive write lock on a background thread (a
    /// stand-in for that lingering process) and asserts `provision_database`
    /// retries and eventually succeeds instead of failing instantly with
    /// `SQLITE_BUSY` ("database is locked") -- which it did before this
    /// connection had `busy_timeout` set (default is 0, no retry).
    #[test]
    fn provision_database_retries_when_transiently_locked() {
        let path = std::env::temp_dir().join(format!(
            "myc_sqlite_provision_locked_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let url = path.to_str().unwrap().to_string();

        provision_database(&url).unwrap();

        let holder_url = url.clone();
        let holder = std::thread::spawn(move || {
            let mut conn = SqliteConnection::establish(&holder_url).unwrap();
            conn.batch_execute("BEGIN EXCLUSIVE;").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1500));
            conn.batch_execute("COMMIT;").unwrap();
        });

        // Give the holder thread time to acquire the exclusive lock first.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let start = std::time::Instant::now();
        provision_database(&url).unwrap();
        let waited = start.elapsed();

        holder.join().unwrap();

        assert!(
            waited >= std::time::Duration::from_millis(1000),
            "expected provision_database to block on the held lock and \
             retry via busy_timeout, but it returned after {waited:?} -- \
             either the lock wasn't actually held, or busy_timeout isn't \
             applied",
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{url}-wal"));
        let _ = std::fs::remove_file(format!("{url}-shm"));
    }

    #[test]
    fn provision_database_creates_parent_dir_and_migrates() {
        let dir = std::env::temp_dir().join(format!(
            "myc_sqlite_provision_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let path = dir.join("nested").join("mycelium.db");
        let url = path.to_str().unwrap();

        provision_database(url).unwrap();

        let mut conn = SqliteConnection::establish(url).unwrap();
        let rows: Vec<TableName> = diesel::sql_query(
            "SELECT name FROM sqlite_master WHERE type = 'table'",
        )
        .load(&mut conn)
        .unwrap();

        assert!(rows.iter().any(|r| r.name == "account"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
