use super::map_assertion_row::map_assertion_row_to_dto;
use crate::{
    models::{config::DbPoolProvider, federation_assertion::AssertionRow},
    schema::federation_assertion_cache as assertion_model,
};

use async_trait::async_trait;
use chrono::Utc;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::{StoredAssertion, PROFILE_ASSERTION_KIND},
    entities::AssertionStoreRead,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AssertionStoreRead)]
pub struct AssertionStoreReadSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl AssertionStoreReadSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn read_err(e: diesel::result::Error) -> MappedErrors {
    fetching_err(format!("Failed to read stored assertions: {e}"))
}

fn to_dtos(
    rows: Vec<AssertionRow>,
) -> Result<Vec<StoredAssertion>, MappedErrors> {
    rows.into_iter().map(map_assertion_row_to_dto).collect()
}

#[async_trait]
impl AssertionStoreRead for AssertionStoreReadSqlDbRepository {
    #[tracing::instrument(name = "assertion_store_list_for_links", skip_all)]
    async fn list_valid_for_links(
        &self,
        link_ids: &[Uuid],
    ) -> Result<Vec<StoredAssertion>, MappedErrors> {
        let rows = assertion_model::table
            .filter(assertion_model::link_id.eq_any(link_ids))
            .filter(assertion_model::kind.eq(PROFILE_ASSERTION_KIND))
            .filter(assertion_model::exp.gt(Utc::now()))
            .select(AssertionRow::as_select())
            .load::<AssertionRow>(&mut *self.connection()?)
            .map_err(read_err)?;

        to_dtos(rows)
    }

    #[tracing::instrument(name = "assertion_store_list", skip_all)]
    async fn list_valid(
        &self,
        peer_id: Option<Uuid>,
    ) -> Result<Vec<StoredAssertion>, MappedErrors> {
        let mut query = assertion_model::table
            .filter(assertion_model::kind.eq(PROFILE_ASSERTION_KIND))
            .filter(assertion_model::exp.gt(Utc::now()))
            .select(AssertionRow::as_select())
            .order(assertion_model::received_at.desc())
            .into_boxed();

        if let Some(peer_id) = peer_id {
            query = query.filter(assertion_model::peer_id.eq(peer_id));
        }

        to_dtos(
            query
                .load::<AssertionRow>(&mut *self.connection()?)
                .map_err(read_err)?,
        )
    }
}
