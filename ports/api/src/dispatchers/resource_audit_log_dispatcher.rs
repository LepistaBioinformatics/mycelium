use crate::models::active_backend_modules::SqlAppModule;
use myc_core::domain::dtos::resource_audit_log::NewResourceAuditLogEvent;
use shaku::HasComponent;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Dispatch resource audit log events
///
/// Spawns a new thread to consume events from the resource audit log
/// channel and persist them one at a time. Never panics on a failed
/// connection-get or insert -- logs and moves on to the next event, same as
/// `webhook_dispatcher`'s per-item error handling.
///
/// Two backend-specific bodies (Postgres / SQLite) follow the same
/// cfg-gated-function-name precedent established by `main.rs`'s
/// `initialize_modules`: same signature, one body per backend feature,
/// mutually exclusive since `postgres-backend`/`standalone` are mutually
/// exclusive features on `mycelium-api`.
#[cfg(any(feature = "postgres-backend", feature = "postgres-only"))]
#[tracing::instrument(name = "resource_audit_log_dispatcher", skip_all)]
pub(crate) async fn resource_audit_log_dispatcher(
    app_modules: Arc<SqlAppModule>,
    mut receiver: mpsc::Receiver<NewResourceAuditLogEvent>,
) {
    use myc_diesel::models::config::DbPoolProvider;
    use myc_diesel::repositories::append_resource_audit_log_row;

    tokio::spawn(async move {
        tracing::info!("Starting resource audit log dispatcher");

        let pool_provider: &dyn DbPoolProvider = app_modules.resolve_ref();

        while let Some(event) = receiver.recv().await {
            let mut conn = match pool_provider.get_pool().get() {
                Ok(conn) => conn,
                Err(err) => {
                    tracing::error!(
                        error = ?err,
                        "resource_audit_log_dispatcher: failed to get db connection"
                    );
                    continue;
                }
            };

            if let Err(err) = append_resource_audit_log_row(&mut conn, &event) {
                tracing::error!(
                    error = ?err,
                    resource_type = ?event.resource_type,
                    resource_id = %event.resource_id,
                    "resource_audit_log_dispatcher: insert failed"
                );
                continue;
            }
        }
    });
}

#[cfg(feature = "standalone")]
#[tracing::instrument(name = "resource_audit_log_dispatcher", skip_all)]
pub(crate) async fn resource_audit_log_dispatcher(
    app_modules: Arc<SqlAppModule>,
    mut receiver: mpsc::Receiver<NewResourceAuditLogEvent>,
) {
    use myc_diesel_sqlite::config::SqliteDbPoolProvider;
    use myc_diesel_sqlite::repositories::resource_audit_log::append_resource_audit_log_row;

    tokio::spawn(async move {
        tracing::info!("Starting resource audit log dispatcher");

        let pool_provider: &dyn SqliteDbPoolProvider =
            app_modules.resolve_ref();

        while let Some(event) = receiver.recv().await {
            let mut conn = match pool_provider.get_pool().get() {
                Ok(conn) => conn,
                Err(err) => {
                    tracing::error!(
                        error = ?err,
                        "resource_audit_log_dispatcher: failed to get db connection"
                    );
                    continue;
                }
            };

            if let Err(err) = append_resource_audit_log_row(&mut conn, &event) {
                tracing::error!(
                    error = ?err,
                    resource_type = ?event.resource_type,
                    resource_id = %event.resource_id,
                    "resource_audit_log_dispatcher: insert failed"
                );
                continue;
            }
        }
    });
}
