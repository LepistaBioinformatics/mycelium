use crate::{
    models::{config::DbPoolProvider, error_code::ErrorCode as ErrorCodeModel},
    schema::error_code as error_code_model,
};

use super::shared::map_model_to_dto;
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
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ErrorCodeUpdating for ErrorCodeUpdatingSqlDbRepository {
    async fn update(
        &self,
        error_code: ErrorCode,
    ) -> Result<UpdatingResponseKind<ErrorCode>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated = diesel::update(
            error_code_model::table
                .filter(error_code_model::prefix.eq(&error_code.prefix))
                .filter(error_code_model::code.eq(error_code.error_number)),
        )
        .set((
            error_code_model::message.eq(&error_code.message),
            error_code_model::details.eq(error_code.details.clone()),
            error_code_model::is_internal.eq(error_code.is_internal),
            error_code_model::is_native.eq(error_code.is_native),
        ))
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
                error_code,
                "Error code not found".to_string(),
            )),
        }
    }
}
