use crate::{
    models::{
        config::DbPoolProvider,
        guest_user_on_account::GuestUserOnAccount as GuestUserOnAccountModel,
    },
    schema::{
        guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::GuestUserDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserDeletion)]
pub struct GuestUserDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestUserDeletion for GuestUserDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_guest_user", skip_all)]
    async fn delete(
        &self,
        guest_role_id: Uuid,
        account_id: Uuid,
        email: String,
    ) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if guest user exists
        let guest_user_on_account = guest_user_on_account_model::table
            .inner_join(guest_user_model::table)
            .filter(
                guest_user_on_account_model::account_id
                    .eq(account_id)
                    .and(guest_user_model::email.eq(&email))
                    .and(guest_user_model::guest_role_id.eq(guest_role_id)),
            )
            .select(GuestUserOnAccountModel::as_select())
            .first::<GuestUserOnAccountModel>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check guest user: {}", e))
            })?;

        match guest_user_on_account {
            Some(guest) => {
                // Delete guest user on account
                diesel::delete(
                    guest_user_on_account_model::table.filter(
                        guest_user_on_account_model::guest_user_id
                            .eq(guest.guest_user_id)
                            .and(
                                guest_user_on_account_model::account_id
                                    .eq(guest.account_id),
                            ),
                    ),
                )
                .execute(conn)
                .map_err(|e| {
                    deletion_err(format!("Failed to delete guest user: {}", e))
                })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                (guest_role_id, account_id),
                "Guest user not found".to_string(),
            )),
        }
    }
}
