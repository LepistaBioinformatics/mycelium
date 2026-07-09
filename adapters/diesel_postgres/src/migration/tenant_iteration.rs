use crate::{
    models::{config::DbPool, tenant::Tenant as TenantModel},
    schema::tenant::{self as tenant_model, dsl as tenant_dsl},
};

use diesel::prelude::*;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use uuid::Uuid;

/// Outcome of processing a single tenant row in a migration pass.
///
/// Shared by both `migrate_dek` and `rotate_kek` so the two adapters
/// classify per-row results consistently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowOutcome {
    Migrated,
    Skipped,
    AlreadyDone,
}

/// Aggregate report produced by a per-tenant migration pass.
#[derive(Debug, Clone, Copy)]
pub struct Summary {
    pub scanned: usize,
    pub migrated: usize,
    pub skipped: usize,
    pub already_done: usize,
    pub dry_run: bool,
}

impl Summary {
    pub(crate) fn new(dry_run: bool) -> Self {
        Self {
            scanned: 0,
            migrated: 0,
            skipped: 0,
            already_done: 0,
            dry_run,
        }
    }

    pub(crate) fn record(&mut self, outcome: RowOutcome) {
        self.scanned += 1;
        match outcome {
            RowOutcome::Migrated => self.migrated += 1,
            RowOutcome::Skipped => self.skipped += 1,
            RowOutcome::AlreadyDone => self.already_done += 1,
        }
    }
}

/// Open a pooled connection for a migration pass.
pub(crate) fn acquire_conn(
    pool: &DbPool,
) -> Result<
    diesel::r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<PgConnection>,
    >,
    MappedErrors,
> {
    pool.get()
        .map_err(|e| execution_err(format!("Failed to get DB connection: {e}")))
}

/// Load every tenant (optionally restricted to a single id) using the
/// canonical ordering so all migrators agree on the row set.
pub(crate) fn load_tenants(
    conn: &mut PgConnection,
    tenant_id: Option<Uuid>,
) -> Result<Vec<TenantModel>, MappedErrors> {
    let Some(tid) = tenant_id else {
        return tenant_dsl::tenant
            .select(TenantModel::as_select())
            .load::<TenantModel>(conn)
            .map_err(|e| {
                execution_err(format!("Failed to load tenants: {e}"))
            });
    };

    tenant_dsl::tenant
        .filter(tenant_model::id.eq(tid))
        .select(TenantModel::as_select())
        .load::<TenantModel>(conn)
        .map_err(|e| execution_err(format!("Failed to load tenant: {e}")))
}
