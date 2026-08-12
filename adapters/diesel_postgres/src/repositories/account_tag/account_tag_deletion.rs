use crate::{
    models::config::DbPoolProvider, schema::account_tag as account_tag_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::AccountTagDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTagDeletion)]
pub struct AccountTagDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl AccountTagDeletion for AccountTagDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_account_tag", skip_all)]
    async fn delete(
        &self,
        tag_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if tag exists
        let tag_exists = account_tag_model::table
            .find(tag_id)
            .select(account_tag_model::id)
            .first::<Uuid>(conn)
            .optional()
            .map_err(|e| deletion_err(format!("Failed to check tag: {}", e)))?;

        match tag_exists {
            Some(_) => {
                // Delete tag
                diesel::delete(account_tag_model::table.find(tag_id))
                    .execute(conn)
                    .map_err(|e| {
                        deletion_err(format!("Failed to delete tag: {}", e))
                    })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                tag_id,
                "Tag not found".to_string(),
            )),
        }
    }
}
