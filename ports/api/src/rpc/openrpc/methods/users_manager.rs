use super::super::schema;
use crate::rpc::{method_names, params};

pub fn methods() -> Vec<serde_json::Value> {
    let account_id_schema =
        schema::param_schema_value::<params::UserManagerAccountIdParams>();

    vec![
        serde_json::json!({
            "name": method_names::USER_MANAGER_ACCOUNT_APPROVE,
            "summary": "Approve account",
            "description": "Approves an account after creation. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::USER_MANAGER_ACCOUNT_DISAPPROVE,
            "summary": "Disapprove account",
            "description": "Disapproves an approved account. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::USER_MANAGER_ACCOUNT_ACTIVATE,
            "summary": "Activate account",
            "description": "Activates an account. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::USER_MANAGER_ACCOUNT_DEACTIVATE,
            "summary": "Deactivate account",
            "description": "Deactivates an account. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::USER_MANAGER_ACCOUNT_ARCHIVE,
            "summary": "Archive account",
            "description": "Sets the target account as archived. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::USER_MANAGER_ACCOUNT_UNARCHIVE,
            "summary": "Unarchive account",
            "description": "Sets the target account as un-archived. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
