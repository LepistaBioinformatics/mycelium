use crate::{
    models::{config::DbPoolProvider, error_code::ErrorCode as ErrorCodeModel},
    schema::error_code as error_code_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ErrorCodeRegistration)]
pub struct ErrorCodeRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ErrorCodeRegistration for ErrorCodeRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_error_code", skip_all)]
    async fn create(
        &self,
        error_code: ErrorCode,
    ) -> Result<CreateResponseKind<ErrorCode>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if error code already exists
        let existing = error_code_model::table
            .filter(error_code_model::prefix.eq(&error_code.prefix))
            .filter(error_code_model::code.eq(error_code.error_number))
            .select(ErrorCodeModel::as_select())
            .first::<ErrorCodeModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!(
                    "Failed to check existing error code: {}",
                    e
                ))
            })?;

        if let Some(record) = existing {
            return Ok(CreateResponseKind::NotCreated(
                self.map_model_to_dto(record),
                "Error code already exists".to_string(),
            ));
        }

        // Create new error code
        let new_error = ErrorCodeModel {
            code: error_code.error_number,
            prefix: error_code.prefix,
            message: error_code.message,
            details: error_code.details,
            is_internal: error_code.is_internal,
            is_native: error_code.is_native,
        };

        let created = diesel::insert_into(error_code_model::table)
            .values(&new_error)
            .get_result::<ErrorCodeModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create error code: {}", e))
            })?;

        Ok(CreateResponseKind::Created(self.map_model_to_dto(created)))
    }
}

impl ErrorCodeRegistrationSqlDbRepository {
    fn map_model_to_dto(&self, model: ErrorCodeModel) -> ErrorCode {
        ErrorCode {
            prefix: model.prefix,
            error_number: model.code,
            code: None,
            message: model.message,
            details: model.details,
            is_internal: model.is_internal,
            is_native: model.is_native,
        }
    }
}
