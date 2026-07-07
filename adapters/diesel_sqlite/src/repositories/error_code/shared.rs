use crate::models::error_code::ErrorCode as ErrorCodeModel;
use myc_core::domain::dtos::error_code::ErrorCode;

pub(super) fn map_model_to_dto(model: ErrorCodeModel) -> ErrorCode {
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
