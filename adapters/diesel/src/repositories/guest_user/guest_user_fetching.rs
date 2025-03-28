use super::shared::map_model_to_dto;
use crate::{
    models::{
        config::DbPoolProvider, guest_role::GuestRole as GuestRoleModel,
        guest_user::GuestUser as GuestUserModel,
    },
    schema::guest_role as guest_role_model,
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
    #[tracing::instrument(name = "list_guest_users", skip_all)]
    async fn list(
        &self,
        account_id: Uuid,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let base_query = guest_user_model::table
            .inner_join(guest_user_on_account_model::table)
            .inner_join(guest_role_model::table)
            .filter(guest_user_on_account_model::account_id.eq(account_id));

        let count_query = base_query.clone();
        let records_query = base_query.clone();

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let records = records_query
            .select((GuestUserModel::as_select(), GuestRoleModel::as_select()))
            .limit(page_size)
            .offset(skip)
            .order(guest_user_model::created.desc())
            .load::<(GuestUserModel, GuestRoleModel)>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch guest users: {}", e))
            })?;

        let count = count_query.count().first::<i64>(conn).map_err(|e| {
            fetching_err(format!("Failed to count guest users: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        Ok(FetchManyResponseKind::FoundPaginated {
            count,
            skip: Some(skip),
            size: Some(page_size),
            records: records
                .into_iter()
                .map(|(user, role)| map_model_to_dto(user, Some(role)))
                .collect(),
        })
    }
}
