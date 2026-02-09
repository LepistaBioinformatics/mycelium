//! OpenRPC method descriptors for guest manager scope (guestRoles).

use super::super::schema;
use crate::rpc::{method_names, params};

pub fn methods() -> Vec<serde_json::Value> {
    let create_guest_role_schema =
        schema::param_schema_value::<params::CreateGuestRoleParams>();
    let list_guest_roles_schema = schema::param_schema_value::<
        params::GuestManagerListGuestRolesParams,
    >();
    let delete_guest_role_schema =
        schema::param_schema_value::<params::DeleteGuestRoleParams>();
    let update_name_desc_schema = schema::param_schema_value::<
        params::UpdateGuestRoleNameAndDescriptionParams,
    >();
    let update_permission_schema =
        schema::param_schema_value::<params::UpdateGuestRolePermissionParams>();
    let insert_role_child_schema =
        schema::param_schema_value::<params::InsertRoleChildParams>();
    let remove_role_child_schema =
        schema::param_schema_value::<params::RemoveRoleChildParams>();

    vec![
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_CREATE,
            "summary": "Create guest role",
            "description": "Creates a guest role. Requires GuestsManager privileges. Permission: 0 = Read, 1 = Write.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": create_guest_role_schema }],
            "result": { "name": "result", "description": "Created or existing guest role (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_LIST,
            "summary": "List guest roles",
            "description": "Lists guest roles with optional filters (name, slug, system) and pagination (pageSize, skip). Requires GuestsManager privileges.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": false, "schema": list_guest_roles_schema }],
            "result": { "name": "result", "description": "List of guest roles (FetchManyResponseKind)", "schema": { "type": "array", "items": { "type": "object" } } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_DELETE,
            "summary": "Delete guest role",
            "description": "Deletes a guest role by ID. Requires GuestsManager privileges.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": delete_guest_role_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_UPDATE_NAME_AND_DESCRIPTION,
            "summary": "Update guest role name and description",
            "description": "Updates name and/or description of a guest role. Requires GuestsManager privileges.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": update_name_desc_schema }],
            "result": { "name": "result", "description": "Updated guest role (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_UPDATE_PERMISSION,
            "summary": "Update guest role permission",
            "description": "Updates permission of a guest role (0 = Read, 1 = Write). Requires GuestsManager privileges.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": update_permission_schema }],
            "result": { "name": "result", "description": "Updated guest role (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_INSERT_ROLE_CHILD,
            "summary": "Insert child role",
            "description": "Adds a child guest role to a parent role. Requires GuestsManager privileges.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": insert_role_child_schema }],
            "result": { "name": "result", "description": "Updated role (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::GUEST_MANAGER_GUEST_ROLES_REMOVE_ROLE_CHILD,
            "summary": "Remove child role",
            "description": "Removes a child guest role from a parent role. Requires GuestsManager privileges.",
            "tags": [{ "name": "guestManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": remove_role_child_schema }],
            "result": { "name": "result", "description": "Updated role (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
