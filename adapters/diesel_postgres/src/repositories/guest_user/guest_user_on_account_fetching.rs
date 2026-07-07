use crate::{
    models::{
        config::DbPoolProvider,
        guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    },
    schema::{guest_user as guest_user_model, guest_user_on_account},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        guest_user_on_account::GuestUserOnAccount,
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestUserOnAccountFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserOnAccountFetching)]
pub struct GuestUserOnAccountFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestUserOnAccountFetching for GuestUserOnAccountFetchingSqlDbRepository {
    #[tracing::instrument(name = "list_by_guest_role_id", skip_all)]
    async fn list_by_guest_role_id(
        &self,
        guest_role_id: Uuid,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<GuestUserOnAccount>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Query guest_user_on_account with join to guest_user
        // Filter by guest_role_id (through guest_user) and account_id
        let records = guest_user_on_account::table
            .inner_join(guest_user_model::table)
            .filter(
                guest_user_model::guest_role_id
                    .eq(guest_role_id)
                    .and(guest_user_on_account::account_id.eq(account_id)),
            )
            .select(GuestUserOnAccountModel::as_select())
            .load::<GuestUserOnAccountModel>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to fetch guest user on account: {}",
                    e
                ))
            })?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        // Map models to DTOs
        let dtos: Vec<GuestUserOnAccount> = records
            .into_iter()
            .map(|model| GuestUserOnAccount {
                guest_user_id: model.guest_user_id,
                account_id: model.account_id,
                created: model
                    .created
                    .and_local_timezone(Local)
                    .unwrap()
                    .with_timezone(&Local),
                permit_flags: model.permit_flags.unwrap_or_default(),
                deny_flags: model.deny_flags.unwrap_or_default(),
            })
            .collect();

        Ok(FetchManyResponseKind::Found(dtos))
    }
}
