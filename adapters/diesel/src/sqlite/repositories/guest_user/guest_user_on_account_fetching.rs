use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    schema::{guest_user, guest_user_on_account},
    types::{
        naive_timestamp_from_text, string_array_from_text, uuid_from_text,
        uuid_to_text,
    },
};

use async_trait::async_trait;
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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
            .inner_join(guest_user::table)
            .filter(
                guest_user::guest_role_id
                    .eq(uuid_to_text(&guest_role_id))
                    .and(
                        guest_user_on_account::account_id
                            .eq(uuid_to_text(&account_id)),
                    ),
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
                guest_user_id: uuid_from_text(&model.guest_user_id).unwrap(),
                account_id: uuid_from_text(&model.account_id).unwrap(),
                created: naive_timestamp_from_text(&model.created)
                    .unwrap()
                    .and_local_timezone(chrono::Local)
                    .unwrap(),
                permit_flags: model
                    .permit_flags
                    .map(|f| string_array_from_text(&f).unwrap())
                    .unwrap_or_default(),
                deny_flags: model
                    .deny_flags
                    .map(|f| string_array_from_text(&f).unwrap())
                    .unwrap_or_default(),
            })
            .collect();

        Ok(FetchManyResponseKind::Found(dtos))
    }
}
