use crate::{
    models::{config::DbPoolProvider, guest_user::GuestUser as GuestUserModel},
    schema::guest_user as guest_user_model,
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
        let guest_user = guest_user_model::table
            .filter(
                guest_user_model::guest_role_id.eq(guest_role_id.to_string()),
            )
            .filter(guest_user_model::email.eq(&email))
            .select(GuestUserModel::as_select())
            .first::<GuestUserModel>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check guest user: {}", e))
            })?;

        match guest_user {
            Some(_) => {
                // Delete guest user
                diesel::delete(
                    guest_user_model::table
                        .filter(
                            guest_user_model::guest_role_id
                                .eq(guest_role_id.to_string()),
                        )
                        .filter(guest_user_model::email.eq(&email)),
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
