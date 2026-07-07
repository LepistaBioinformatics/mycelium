use crate::sqlite::{
    config::SqliteDbPoolProvider, schema::webhook, types::uuid_to_text,
};

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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl WebHookDeletion for WebHookDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_webhook", skip_all)]
    async fn delete(
        &self,
        hook_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let hook_id_text = uuid_to_text(&hook_id);

        // Check if webhook exists
        let exists = webhook::table
            .find(&hook_id_text)
            .select(webhook::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check webhook: {}", e))
            })?;

        match exists {
            Some(_) => {
                // Delete webhook
                diesel::delete(webhook::table.find(&hook_id_text))
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
