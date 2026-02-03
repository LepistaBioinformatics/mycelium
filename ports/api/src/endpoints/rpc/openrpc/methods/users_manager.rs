use super::super::schema;
use crate::endpoints::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let account_id_schema =
        schema::param_schema_value::<params::UserManagerAccountIdParams>();

    vec![
        serde_json::json!({
            "name": "userManager.account.approveAccount",
            "summary": "Approve account",
            "description": "Approves an account after creation. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "userManager.account.disapproveAccount",
            "summary": "Disapprove account",
            "description": "Disapproves an approved account. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "userManager.account.activateAccount",
            "summary": "Activate account",
            "description": "Activates an account. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "userManager.account.deactivateAccount",
            "summary": "Deactivate account",
            "description": "Deactivates an account. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "userManager.account.archiveAccount",
            "summary": "Archive account",
            "description": "Sets the target account as archived. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "userManager.account.unarchiveAccount",
            "summary": "Unarchive account",
            "description": "Sets the target account as un-archived. Requires UsersManager privileges.",
            "tags": [{ "name": "userManager" }, { "name": "account" }],
            "params": [{ "name": "params", "required": true, "schema": account_id_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
