use super::map_link_row::map_link_row_to_dto;
use crate::{
    models::{config::DbPoolProvider, federation_link::IdentityLinkRow},
    schema::federation_identity_link as link_model,
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::{IdentityLink, LinkStatus, SubjectRef},
    entities::IdentityLinkRead,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = IdentityLinkRead)]
pub struct IdentityLinkReadSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl IdentityLinkReadSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn found<U>(
    row: Option<IdentityLinkRow>,
    key: U,
) -> Result<FetchResponseKind<IdentityLink, U>, MappedErrors> {
    let Some(row) = row else {
        return Ok(FetchResponseKind::NotFound(Some(key)));
    };

    Ok(FetchResponseKind::Found(map_link_row_to_dto(row)?))
}

fn read_err(e: diesel::result::Error) -> MappedErrors {
    fetching_err(format!("Failed to read identity link: {e}"))
}

#[async_trait]
impl IdentityLinkRead for IdentityLinkReadSqlDbRepository {
    #[tracing::instrument(name = "identity_link_get", skip_all)]
    async fn get(
        &self,
        link_id: Uuid,
    ) -> Result<FetchResponseKind<IdentityLink, Uuid>, MappedErrors> {
        let row = link_model::table
            .find(link_id)
            .select(IdentityLinkRow::as_select())
            .first::<IdentityLinkRow>(&mut *self.connection()?)
            .optional()
            .map_err(read_err)?;

        found(row, link_id)
    }

    #[tracing::instrument(name = "identity_link_find_live", skip_all)]
    async fn find_live(
        &self,
        local_user_id: Uuid,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<IdentityLink, Uuid>, MappedErrors> {
        let row = link_model::table
            .filter(link_model::local_user_id.eq(local_user_id))
            .filter(link_model::peer_id.eq(peer_id))
            .filter(link_model::status.ne(LinkStatus::Revoked.as_str()))
            .select(IdentityLinkRow::as_select())
            .first::<IdentityLinkRow>(&mut *self.connection()?)
            .optional()
            .map_err(read_err)?;

        found(row, peer_id)
    }

    #[tracing::instrument(name = "identity_link_find_by_local_ref", skip_all)]
    async fn find_by_local_ref(
        &self,
        peer_id: Uuid,
        local_subject_ref: SubjectRef,
    ) -> Result<FetchResponseKind<IdentityLink, SubjectRef>, MappedErrors> {
        let row = link_model::table
            .filter(link_model::peer_id.eq(peer_id))
            .filter(
                link_model::local_subject_ref.eq(local_subject_ref.as_str()),
            )
            .select(IdentityLinkRow::as_select())
            .first::<IdentityLinkRow>(&mut *self.connection()?)
            .optional()
            .map_err(read_err)?;

        found(row, local_subject_ref)
    }

    #[tracing::instrument(name = "identity_link_list", skip_all)]
    async fn list(
        &self,
        local_user_id: Option<Uuid>,
    ) -> Result<Vec<IdentityLink>, MappedErrors> {
        let mut query = link_model::table
            .select(IdentityLinkRow::as_select())
            .order(link_model::created.desc())
            .into_boxed();

        if let Some(user_id) = local_user_id {
            query = query.filter(link_model::local_user_id.eq(user_id));
        }

        query
            .load::<IdentityLinkRow>(&mut *self.connection()?)
            .map_err(read_err)?
            .into_iter()
            .map(map_link_row_to_dto)
            .collect()
    }
}
