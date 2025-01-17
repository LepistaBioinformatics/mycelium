use crate::{
    models::config::DbPoolProvider, schema::guest_role as guest_role_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::GuestRoleDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleDeletion)]
pub struct GuestRoleDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestRoleDeletion for GuestRoleDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_guest_role", skip_all)]
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if role exists
        let role_exists = guest_role_model::table
            .find(id.to_string())
            .select(guest_role_model::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check role: {}", e))
            })?;

        match role_exists {
            Some(_) => {
                // Delete role
                diesel::delete(guest_role_model::table.find(id.to_string()))
                    .execute(conn)
                    .map_err(|e| {
                        deletion_err(format!("Failed to delete role: {}", e))
                    })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                id,
                "Role not found".to_string(),
            )),
        }
    }
}
