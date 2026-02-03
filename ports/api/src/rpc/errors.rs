use super::types::{self, JsonRpcError};
use mycelium_base::utils::errors::MappedErrors;

/// Builds a JSON-RPC "invalid params" error (-32602).
pub fn invalid_params(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError {
        code: types::codes::INVALID_PARAMS,
        message: message.into(),
        data: None,
    }
}

/// "params required" invalid params error.
pub fn params_required() -> JsonRpcError {
    invalid_params("params required")
}

/// Forbidden error for operations restricted to account owners.
pub fn forbidden_owner_only() -> JsonRpcError {
    JsonRpcError {
        code: types::codes::FORBIDDEN,
        message: "Invalid operation. Operation restricted to account owners."
            .to_string(),
        data: None,
    }
}

pub fn mapped_errors_to_jsonrpc_error(err: MappedErrors) -> JsonRpcError {
    let code_string = err.code().to_string();
    let message = err.to_string();

    use myc_core::domain::dtos::native_error_codes::NativeErrorCodes::*;

    let code =
        if err.is_in(vec![MYC00001, MYC00004, MYC00007, MYC00010, MYC00012]) {
            types::codes::INTERNAL_ERROR
        } else if err.is_in(vec![
            MYC00002, MYC00003, MYC00014, MYC00015, MYC00017, MYC00018,
        ]) {
            409
        } else if err.is_in(vec![
            MYC00005, MYC00006, MYC00008, MYC00009, MYC00011, MYC00013,
            MYC00016, MYC00021, MYC00022, MYC00023,
        ]) {
            types::codes::INVALID_PARAMS
        } else if err.is_in(vec![MYC00019, MYC00020]) {
            types::codes::FORBIDDEN
        } else {
            types::codes::INTERNAL_ERROR
        };

    JsonRpcError {
        code,
        message: message.clone(),
        data: Some(serde_json::json!({ "code": code_string })),
    }
}
