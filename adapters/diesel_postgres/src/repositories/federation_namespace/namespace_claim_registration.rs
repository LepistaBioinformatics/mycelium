use crate::{
    models::config::DbPoolProvider,
    schema::federation_namespace_claim as claim_model,
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::NamespaceClaim, entities::NamespaceClaimRegistration,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = NamespaceClaimRegistration)]
pub struct NamespaceClaimRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl NamespaceClaimRegistrationSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn claim_err(e: diesel::result::Error) -> MappedErrors {
    creation_err(format!("Failed to claim a foreign id: {e}"))
}

#[async_trait]
impl NamespaceClaimRegistration for NamespaceClaimRegistrationSqlDbRepository {
    #[tracing::instrument(name = "namespace_claim", skip_all)]
    async fn claim(&self, claim: NamespaceClaim) -> Result<Uuid, MappedErrors> {
        let connection = &mut *self.connection()?;

        // A concurrent first claim makes this insert a no-op; the owner read
        // afterwards is then the other peer
        diesel::insert_into(claim_model::table)
            .values((
                claim_model::foreign_id.eq(claim.foreign_id),
                claim_model::peer_id.eq(claim.peer_id),
                claim_model::kind.eq(claim.kind.as_str()),
            ))
            .on_conflict(claim_model::foreign_id)
            .do_nothing()
            .execute(connection)
            .map_err(claim_err)?;

        claim_model::table
            .find(claim.foreign_id)
            .select(claim_model::peer_id)
            .first::<Uuid>(connection)
            .map_err(claim_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        federation_link::live_link_schema::{active_link, live_pool, provider},
        federation_namespace::LocalNamespaceProbeSqlDbRepository,
    };

    use diesel::connection::SimpleConnection;
    use myc_core::domain::{
        dtos::federation::ForeignIdKind, entities::LocalNamespaceProbe,
    };

    fn claim_of(foreign_id: Uuid, peer_id: Uuid) -> NamespaceClaim {
        NamespaceClaim {
            foreign_id,
            peer_id,
            kind: ForeignIdKind::Tenant,
        }
    }

    #[tokio::test]
    async fn the_first_claim_wins() {
        let Some(pool) = live_pool() else { return };
        let repo = NamespaceClaimRegistrationSqlDbRepository {
            db_config: provider(pool.clone()),
        };
        let (_, first) = active_link(&pool);
        let (_, second) = active_link(&pool);
        let foreign_id = Uuid::new_v4();

        let owner = repo.claim(claim_of(foreign_id, first)).await.unwrap();
        let again = repo.claim(claim_of(foreign_id, second)).await.unwrap();

        assert_eq!(owner, first);
        assert_eq!(again, first);
    }

    #[tokio::test]
    async fn the_probe_finds_local_tenants_and_accounts_only() {
        let Some(pool) = live_pool() else { return };
        let probe = LocalNamespaceProbeSqlDbRepository {
            db_config: provider(pool.clone()),
        };
        let (tenant, account, foreign) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        pool.get()
            .unwrap()
            .batch_execute(&format!(
                "INSERT INTO tenant (id) VALUES ('{tenant}');
                 INSERT INTO account (id) VALUES ('{account}');"
            ))
            .unwrap();

        let mut found =
            probe.existing(&[tenant, account, foreign]).await.unwrap();
        found.sort();
        let mut expected = vec![tenant, account];
        expected.sort();

        assert_eq!(found, expected);
        assert!(probe.existing(&[]).await.unwrap().is_empty());
    }
}
