use myc_diesel::repositories::{
    DieselDbPoolProvider, DieselDbPoolProviderParameters,
    ResourceAuditLogRegistrationSqlDbRepository,
    ResourceAuditLogRegistrationSqlDbRepositoryParameters, SqlAppModule,
};
use tokio::sync::mpsc;

/// The SQL module over `database_url`, for commands that write no audit
/// event
///
/// Building the module requires the audit channel sender; with no dispatcher
/// in the CLI its receiver is dropped, so a command that did emit an event
/// would get an error instead of silently losing it.
///
pub(crate) fn connect_sql_module(database_url: &str) -> SqlAppModule {
    let (audit_sender, _) = mpsc::channel(1);

    SqlAppModule::builder()
        .with_component_parameters::<DieselDbPoolProvider>(
            DieselDbPoolProviderParameters {
                pool: DieselDbPoolProvider::new(database_url),
            },
        )
        .with_component_parameters::<ResourceAuditLogRegistrationSqlDbRepository>(
            ResourceAuditLogRegistrationSqlDbRepositoryParameters {
                sender: audit_sender,
            },
        )
        .build()
}
