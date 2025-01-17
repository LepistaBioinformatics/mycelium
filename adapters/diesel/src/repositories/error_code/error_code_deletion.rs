use crate::{
    models::config::DbPoolProvider, schema::error_code as error_code_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::ErrorCodeDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ErrorCodeDeletion)]
pub struct ErrorCodeDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ErrorCodeDeletion for ErrorCodeDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_error_code", skip_all)]
    async fn delete(
        &self,
        prefix: String,
        code: i32,
    ) -> Result<DeletionResponseKind<(String, i32)>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if error code exists
        let error_exists = error_code_model::table
            .filter(error_code_model::prefix.eq(&prefix))
            .filter(error_code_model::code.eq(code))
            .select(error_code_model::code)
            .first::<i32>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check error code: {}", e))
            })?;

        match error_exists {
            Some(_) => {
                // Delete error code
                diesel::delete(
                    error_code_model::table
                        .filter(error_code_model::prefix.eq(&prefix))
                        .filter(error_code_model::code.eq(code)),
                )
                .execute(conn)
                .map_err(|e| {
                    deletion_err(format!("Failed to delete error code: {}", e))
                })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                (prefix, code),
                "Error code not found".to_string(),
            )),
        }
    }
}
