use super::shared::map_model_to_dto;
use crate::{
    models::{config::DbPoolProvider, guest_role::GuestRole as GuestRoleModel},
    schema::guest_role as guest_role_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{guest_role::GuestRole, native_error_codes::NativeErrorCodes},
    entities::GuestRoleFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleFetching)]
pub struct GuestRoleFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestRoleFetching for GuestRoleFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let role = guest_role_model::table
            .find(id)
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch role: {}", e))
            })?;

        match role {
            Some(record) => {
                Ok(FetchResponseKind::Found(map_model_to_dto(record)))
            }
            None => Ok(FetchResponseKind::NotFound(Some(id))),
        }
    }

    async fn list(
        &self,
        name: Option<String>,
    ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = guest_role_model::table.into_boxed();

        // Apply name filter if provided
        if let Some(name) = name {
            query = query
                .filter(guest_role_model::name.ilike(format!("%{}%", name)));
        }

        let records = query
            .select(GuestRoleModel::as_select())
            .load::<GuestRoleModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch roles: {}", e))
            })?;

        Ok(FetchManyResponseKind::Found(
            records.into_iter().map(|r| map_model_to_dto(r)).collect(),
        ))
    }
}
