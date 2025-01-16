use super::shared::map_model_to_dto;
use crate::{
    models::{config::DbPoolProvider, guest_user::GuestUser as GuestUserModel},
    schema::guest_user as guest_user_model,
    schema::guest_user_on_account as guest_user_on_account_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{guest_user::GuestUser, native_error_codes::NativeErrorCodes},
    entities::GuestUserFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserFetching)]
pub struct GuestUserFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestUserFetching for GuestUserFetchingSqlDbRepository {
    async fn list(
        &self,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let records = guest_user_model::table
            .inner_join(guest_user_on_account_model::table)
            .filter(guest_user_on_account_model::account_id.eq(account_id))
            .select(GuestUserModel::as_select())
            .load::<GuestUserModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch guest users: {}", e))
            })?;

        Ok(FetchManyResponseKind::Found(
            records.into_iter().map(|r| map_model_to_dto(r)).collect(),
        ))
    }
}
