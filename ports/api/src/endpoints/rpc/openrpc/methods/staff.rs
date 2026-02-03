use super::super::schema;
use crate::endpoints::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let upgrade_schema =
        schema::param_schema_value::<params::UpgradeAccountPrivilegesParams>();
    let downgrade_schema = schema::param_schema_value::<
        params::DowngradeAccountPrivilegesParams,
    >();

    vec![
        serde_json::json!({
            "name": "staff.accounts.upgradeAccountPrivileges",
            "summary": "Upgrade account privileges",
            "description": "Increases permissions of the account. Target type (to): Staff or Manager. Requires Staff privileges.",
            "tags": [{ "name": "staff" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": upgrade_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "staff.accounts.downgradeAccountPrivileges",
            "summary": "Downgrade account privileges",
            "description": "Decreases permissions of the account. Target type (to): Manager or User. Requires Staff privileges.",
            "tags": [{ "name": "staff" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": downgrade_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
