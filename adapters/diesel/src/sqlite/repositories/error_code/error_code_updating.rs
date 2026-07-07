use super::shared::map_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::error_code::ErrorCode as ErrorCodeModel, schema::error_code,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ErrorCodeUpdating)]
pub struct ErrorCodeUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl ErrorCodeUpdating for ErrorCodeUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_error_code", skip_all)]
    async fn update(
        &self,
        error_code_dto: ErrorCode,
    ) -> Result<UpdatingResponseKind<ErrorCode>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated = diesel::update(
            error_code::table.filter(
                error_code::prefix
                    .eq(error_code_dto.prefix.clone())
                    .and(error_code::code.eq(error_code_dto.error_number)),
            ),
        )
        .set((
            error_code::message.eq(&error_code_dto.message),
            error_code::details.eq(error_code_dto.details.clone()),
            error_code::is_internal.eq(error_code_dto.is_internal),
            error_code::is_native.eq(error_code_dto.is_native),
        ))
        .returning(ErrorCodeModel::as_returning())
        .get_result::<ErrorCodeModel>(conn)
        .optional()
        .map_err(|e| {
            updating_err(format!("Failed to update error code: {}", e))
        })?;

        match updated {
            Some(record) => {
                Ok(UpdatingResponseKind::Updated(map_model_to_dto(record)))
            }
            None => Ok(UpdatingResponseKind::NotUpdated(
                error_code_dto,
                "Error code not found".to_string(),
            )),
        }
    }
}
