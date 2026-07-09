use crate::{
    config::SqliteDbPoolProvider, schema::tenant_tag, types::uuid_to_text,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::TenantTagDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantTagDeletion)]
pub struct TenantTagDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TenantTagDeletion for TenantTagDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_tenant_tag", skip_all)]
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let id_text = uuid_to_text(&id);

        // Check if tag exists
        let exists = tenant_tag::table
            .find(&id_text)
            .select(tenant_tag::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| deletion_err(format!("Failed to check tag: {}", e)))?;

        match exists {
            Some(_) => {
                // Delete tag
                diesel::delete(tenant_tag::table.find(&id_text))
                    .execute(conn)
                    .map_err(|e| {
                        deletion_err(format!("Failed to delete tag: {}", e))
                    })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                id,
                "Tag not found".to_string(),
            )),
        }
    }
}
