use crate::{models::config::DbPoolProvider, schema::webhook as webhook_model};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::WebHookDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookDeletion)]
pub struct WebHookDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl WebHookDeletion for WebHookDeletionSqlDbRepository {
    async fn delete(
        &self,
        hook_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if webhook exists
        let exists = webhook_model::table
            .find(hook_id)
            .select(webhook_model::id)
            .first::<Uuid>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check webhook: {}", e))
            })?;

        match exists {
            Some(_) => {
                // Delete webhook
                diesel::delete(webhook_model::table.find(hook_id))
                    .execute(conn)
                    .map_err(|e| {
                        deletion_err(format!("Failed to delete webhook: {}", e))
                    })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                hook_id,
                "Webhook not found".to_string(),
            )),
        }
    }
}
