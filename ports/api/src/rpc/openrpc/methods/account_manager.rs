use super::super::schema;
use crate::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let guest_to_children_account_schema =
        schema::param_schema_value::<params::GuestToChildrenAccountParams>();
    let list_guest_roles_schema =
        schema::param_schema_value::<params::ListGuestRolesParams>();
    let fetch_guest_role_details_schema =
        schema::param_schema_value::<params::FetchGuestRoleDetailsParams>();

    vec![
        serde_json::json!({
            "name": "accountManager.guests.guestToChildrenAccount",
            "summary": "Guest user to children account",
            "description": "Adds a guest user to an account under the given guest role (child role). Requires account manager privileges on the tenant. Tenant ID, account ID and role ID must be provided.",
            "tags": [{ "name": "accountManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": guest_to_children_account_schema }],
            "result": { "name": "result", "description": "Created or existing guest user (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "accountManager.guestRoles.listGuestRoles",
            "summary": "List guest roles",
            "description": "Lists guest roles with optional filters (name, slug, system) and pagination (pageSize, skip). Optional tenant ID to scope. Requires account manager privileges.",
            "tags": [{ "name": "accountManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": false, "schema": list_guest_roles_schema }],
            "result": { "name": "result", "description": "List of guest roles (FetchManyResponseKind)", "schema": { "type": "array", "items": { "type": "object" } } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "accountManager.guestRoles.fetchGuestRoleDetails",
            "summary": "Fetch guest role details",
            "description": "Returns details for a guest role by ID. Optional tenant ID to scope. Requires account manager privileges.",
            "tags": [{ "name": "accountManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": fetch_guest_role_details_schema }],
            "result": { "name": "result", "description": "Guest role or not found (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
