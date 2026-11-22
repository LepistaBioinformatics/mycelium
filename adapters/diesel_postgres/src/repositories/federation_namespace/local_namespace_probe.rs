use crate::{
    models::config::DbPoolProvider,
    schema::{account as account_model, tenant as tenant_model},
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::entities::LocalNamespaceProbe;
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = LocalNamespaceProbe)]
pub struct LocalNamespaceProbeSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl LocalNamespaceProbeSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn probe_err(e: diesel::result::Error) -> MappedErrors {
    fetching_err(format!("Failed to probe local ids: {e}"))
}

#[async_trait]
impl LocalNamespaceProbe for LocalNamespaceProbeSqlDbRepository {
    #[tracing::instrument(name = "local_namespace_probe", skip_all)]
    async fn existing(&self, ids: &[Uuid]) -> Result<Vec<Uuid>, MappedErrors> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        tenant_model::table
            .filter(tenant_model::id.eq_any(ids))
            .select(tenant_model::id)
            .union(
                account_model::table
                    .filter(account_model::id.eq_any(ids))
                    .select(account_model::id),
            )
            .load::<Uuid>(&mut *self.connection()?)
            .map_err(probe_err)
    }
}
