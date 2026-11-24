use super::map_peer_row::{map_key_row_to_dto, map_peer_row_to_dto};
use crate::{
    models::{
        config::DbPoolProvider,
        federation_peer::{FederationPeerKeyRow, FederationPeerRow},
    },
    schema::{federation_peer as peer_model, federation_peer_key as key_model},
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::federation::{FederationPeer, PeerKey},
    entities::FederationPeerRead,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = FederationPeerRead)]
pub struct FederationPeerReadSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl FederationPeerReadSqlDbRepository {
    fn get_one(
        &self,
        filter: impl FnOnce(
            peer_model::BoxedQuery<'static, diesel::pg::Pg>,
        )
            -> peer_model::BoxedQuery<'static, diesel::pg::Pg>,
        missing: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })?;

        let row = filter(peer_model::table.into_boxed())
            .select(FederationPeerRow::as_select())
            .first::<FederationPeerRow>(conn)
            .optional()
            .map_err(|e| fetching_err(format!("Failed to fetch peer: {e}")))?;

        match row {
            None => Ok(FetchResponseKind::NotFound(Some(missing))),
            Some(row) => {
                Ok(FetchResponseKind::Found(map_peer_row_to_dto(row)?))
            }
        }
    }
}

#[async_trait]
impl FederationPeerRead for FederationPeerReadSqlDbRepository {
    #[tracing::instrument(name = "federation_peer_get", skip_all)]
    async fn get(
        &self,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        self.get_one(|query| query.filter(peer_model::id.eq(peer_id)), peer_id)
    }

    #[tracing::instrument(
        name = "federation_peer_get_by_instance_id",
        skip_all
    )]
    async fn get_by_instance_id(
        &self,
        instance_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        self.get_one(
            |query| query.filter(peer_model::instance_id.eq(instance_id)),
            instance_id,
        )
    }

    #[tracing::instrument(name = "federation_peer_list", skip_all)]
    async fn list(
        &self,
    ) -> Result<FetchManyResponseKind<FederationPeer>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })?;

        let rows = peer_model::table
            .order(peer_model::created.asc())
            .select(FederationPeerRow::as_select())
            .load::<FederationPeerRow>(conn)
            .map_err(|e| fetching_err(format!("Failed to list peers: {e}")))?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(
            rows.into_iter()
                .map(map_peer_row_to_dto)
                .collect::<Result<_, _>>()?,
        ))
    }

    #[tracing::instrument(name = "federation_peer_list_keys", skip_all)]
    async fn list_keys(
        &self,
        peer_id: Uuid,
    ) -> Result<Vec<PeerKey>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })?;

        let rows = key_model::table
            .filter(key_model::peer_id.eq(peer_id))
            .order(key_model::kid.asc())
            .select(FederationPeerKeyRow::as_select())
            .load::<FederationPeerKeyRow>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to list peer keys: {e}"))
            })?;

        Ok(rows.into_iter().map(map_key_row_to_dto).collect())
    }
}
