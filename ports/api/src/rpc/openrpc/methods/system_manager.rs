use super::super::schema;
use crate::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let register_error_code_schema =
        schema::param_schema_value::<params::RegisterErrorCodeParams>();
    let list_error_codes_schema =
        schema::param_schema_value::<params::ListErrorCodesParams>();
    let get_error_code_schema =
        schema::param_schema_value::<params::GetErrorCodeParams>();
    let update_error_code_schema = schema::param_schema_value::<
        params::UpdateErrorCodeMessageAndDetailsParams,
    >();
    let delete_error_code_schema =
        schema::param_schema_value::<params::DeleteErrorCodeParams>();
    let register_webhook_schema =
        schema::param_schema_value::<params::RegisterWebhookParams>();
    let list_webhooks_schema =
        schema::param_schema_value::<params::ListWebhooksParams>();
    let update_webhook_schema =
        schema::param_schema_value::<params::UpdateWebhookParams>();
    let delete_webhook_schema =
        schema::param_schema_value::<params::DeleteWebhookParams>();

    vec![
        serde_json::json!({
            "name": "systemManager.errorCodes.registerErrorCode",
            "summary": "Register error code",
            "description": "Registers a new error code. Requires SystemManager privileges.",
            "tags": [{ "name": "systemManager" }, { "name": "errorCodes" }],
            "params": [{ "name": "params", "required": true, "schema": register_error_code_schema }],
            "result": { "name": "result", "description": "Created error code", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.errorCodes.listErrorCodes",
            "summary": "List error codes",
            "description": "Lists error codes with optional filters (prefix, code, isInternal) and pagination.",
            "tags": [{ "name": "systemManager" }, { "name": "errorCodes" }],
            "params": [{ "name": "params", "required": false, "schema": list_error_codes_schema }],
            "result": { "name": "result", "description": "List or paginated records (FetchManyResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.errorCodes.getErrorCode",
            "summary": "Get error code",
            "description": "Returns an error code by prefix and code.",
            "tags": [{ "name": "systemManager" }, { "name": "errorCodes" }],
            "params": [{ "name": "params", "required": true, "schema": get_error_code_schema }],
            "result": { "name": "result", "description": "Error code or null (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.errorCodes.updateErrorCodeMessageAndDetails",
            "summary": "Update error code message and details",
            "description": "Updates message and details of an error code.",
            "tags": [{ "name": "systemManager" }, { "name": "errorCodes" }],
            "params": [{ "name": "params", "required": true, "schema": update_error_code_schema }],
            "result": { "name": "result", "description": "Updated error code", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.errorCodes.deleteErrorCode",
            "summary": "Delete error code",
            "description": "Deletes an error code by prefix and code.",
            "tags": [{ "name": "systemManager" }, { "name": "errorCodes" }],
            "params": [{ "name": "params", "required": true, "schema": delete_error_code_schema }],
            "result": { "name": "result", "description": "null on success", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.webhooks.registerWebhook",
            "summary": "Register webhook",
            "description": "Registers a webhook. Method must be POST, PUT, PATCH or DELETE. Requires SystemManager privileges.",
            "tags": [{ "name": "systemManager" }, { "name": "webhooks" }],
            "params": [{ "name": "params", "required": true, "schema": register_webhook_schema }],
            "result": { "name": "result", "description": "Created webhook (CreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.webhooks.listWebhooks",
            "summary": "List webhooks",
            "description": "Lists webhooks with optional filters (name, trigger) and pagination.",
            "tags": [{ "name": "systemManager" }, { "name": "webhooks" }],
            "params": [{ "name": "params", "required": false, "schema": list_webhooks_schema }],
            "result": { "name": "result", "description": "List or paginated records (FetchManyResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.webhooks.updateWebhook",
            "summary": "Update webhook",
            "description": "Updates a webhook (name, description, secret, isActive).",
            "tags": [{ "name": "systemManager" }, { "name": "webhooks" }],
            "params": [{ "name": "params", "required": true, "schema": update_webhook_schema }],
            "result": { "name": "result", "description": "Updated webhook (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "systemManager.webhooks.deleteWebhook",
            "summary": "Delete webhook",
            "description": "Deletes a webhook by ID.",
            "tags": [{ "name": "systemManager" }, { "name": "webhooks" }],
            "params": [{ "name": "params", "required": true, "schema": delete_webhook_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
