use super::map_link_request_rows::{found, map_request_row};
use crate::{
    models::{config::DbPoolProvider, federation_link::LinkRequestRow},
    schema::federation_link_request as request_model,
};

use async_trait::async_trait;
use chrono::Utc;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::{LinkRequest, NewLinkRequest},
    entities::LinkRequestRegistration,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{creation_err, updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = LinkRequestRegistration)]
pub struct LinkRequestRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl LinkRequestRegistrationSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn update_err(e: diesel::result::Error) -> MappedErrors {
    updating_err(format!("Failed to update link request: {e}"))
}

#[async_trait]
impl LinkRequestRegistration for LinkRequestRegistrationSqlDbRepository {
    #[tracing::instrument(name = "link_request_create", skip_all)]
    async fn create(
        &self,
        request: NewLinkRequest,
    ) -> Result<LinkRequest, MappedErrors> {
        diesel::insert_into(request_model::table)
            .values((
                request_model::local_user_id.eq(request.local_user_id),
                request_model::peer_id.eq(request.peer_id),
                request_model::secret_hash.eq(&request.secret_hash),
                request_model::expires_at
                    .eq(request.expires_at.with_timezone(&Utc)),
            ))
            .returning(LinkRequestRow::as_returning())
            .get_result::<LinkRequestRow>(&mut *self.connection()?)
            .map(map_request_row)
            .map_err(|e| creation_err(format!("Failed to create request: {e}")))
    }

    #[tracing::instrument(name = "link_request_get_open", skip_all)]
    async fn get_open(
        &self,
        request_id: Uuid,
        secret_hash: String,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors> {
        let row = request_model::table
            .find(request_id)
            .filter(request_model::secret_hash.eq(secret_hash))
            .filter(request_model::redirected_at.is_null())
            .filter(request_model::expires_at.gt(Utc::now()))
            .select(LinkRequestRow::as_select())
            .first::<LinkRequestRow>(&mut *self.connection()?)
            .optional()
            .map_err(update_err)?;

        found(row, request_id, map_request_row)
    }

    #[tracing::instrument(name = "link_request_redirect", skip_all)]
    async fn redirect(
        &self,
        request_id: Uuid,
        secret_hash: String,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors> {
        let row = diesel::update(
            request_model::table
                .find(request_id)
                .filter(request_model::secret_hash.eq(secret_hash))
                .filter(request_model::redirected_at.is_null())
                .filter(request_model::expires_at.gt(Utc::now())),
        )
        .set(request_model::redirected_at.eq(Some(Utc::now())))
        .returning(LinkRequestRow::as_returning())
        .get_result::<LinkRequestRow>(&mut *self.connection()?)
        .optional()
        .map_err(update_err)?;

        found(row, request_id, map_request_row)
    }

    #[tracing::instrument(name = "link_request_consume", skip_all)]
    async fn consume(
        &self,
        request_id: Uuid,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors> {
        let row = diesel::update(
            request_model::table
                .find(request_id)
                .filter(request_model::peer_id.eq(peer_id))
                .filter(request_model::redirected_at.is_not_null())
                .filter(request_model::consumed_at.is_null())
                .filter(request_model::expires_at.gt(Utc::now())),
        )
        .set(request_model::consumed_at.eq(Some(Utc::now())))
        .returning(LinkRequestRow::as_returning())
        .get_result::<LinkRequestRow>(&mut *self.connection()?)
        .optional()
        .map_err(update_err)?;

        found(row, request_id, map_request_row)
    }

    #[tracing::instrument(name = "link_request_purge_expired", skip_all)]
    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        diesel::delete(
            request_model::table
                .filter(request_model::expires_at.le(Utc::now())),
        )
        .execute(&mut *self.connection()?)
        .map(|deleted| deleted as u64)
        .map_err(update_err)
    }
}
