use diesel::SqliteConnection;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, MigrationHarness,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};

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

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_types::Text;
    use diesel::{Connection, QueryableByName, RunQueryDsl};

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
}
