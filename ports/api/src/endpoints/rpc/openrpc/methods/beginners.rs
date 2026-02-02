//! OpenRPC method descriptors for beginners scope (beginners.accounts.*).

use super::super::schema;
use crate::endpoints::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let create_default_account_schema =
        schema::param_schema_value::<params::CreateDefaultAccountParams>();
    let update_own_account_name_schema =
        schema::param_schema_value::<params::UpdateOwnAccountNameParams>();
    let delete_my_account_schema =
        schema::param_schema_value::<params::DeleteMyAccountParams>();

    vec![
        serde_json::json!({
            "name": "beginners.accounts.createDefaultAccount",
            "summary": "Create a user-related account",
            "description": "Creates an account for a physical person. Uses credentials from the request (multi-identity provider).",
            "tags": [{ "name": "beginners" }, { "name": "accounts" }],
            "params": [{ "name": "params", "description": "Creation parameters", "required": true, "schema": create_default_account_schema }],
            "result": { "name": "result", "description": "Created or existing account", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32600, "message": "Invalid request" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.accounts.getMyAccountDetails",
            "summary": "Get my account details",
            "description": "Returns the details of the account associated with the current user.",
            "tags": [{ "name": "beginners" }, { "name": "accounts" }],
            "params": [],
            "result": { "name": "result", "description": "Account or not found (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.accounts.updateOwnAccountName",
            "summary": "Update account name",
            "description": "Updates the account name. Restricted to the account owner (accountId must match authenticated user's account).",
            "tags": [{ "name": "beginners" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": update_own_account_name_schema }],
            "result": { "name": "result", "description": "Updated or not modified (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.accounts.deleteMyAccount",
            "summary": "Delete my account",
            "description": "Deletes the account associated with the current user. Restricted to the account owner (accountId must match authenticated user's account).",
            "tags": [{ "name": "beginners" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": delete_my_account_schema }],
            "result": { "name": "result", "description": "Deletion result (DeletionResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
