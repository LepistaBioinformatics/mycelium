//! OpenRPC method descriptors for managers scope (managers.*).

use super::super::schema;
use crate::endpoints::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let create_system_account_schema =
        schema::param_schema_value::<params::CreateSystemAccountParams>();
    let create_tenant_schema =
        schema::param_schema_value::<params::CreateTenantParams>();
    let list_tenant_schema =
        schema::param_schema_value::<params::ListTenantParams>();
    let delete_tenant_schema =
        schema::param_schema_value::<params::DeleteTenantParams>();
    let include_tenant_owner_schema =
        schema::param_schema_value::<params::IncludeTenantOwnerParams>();
    let exclude_tenant_owner_schema =
        schema::param_schema_value::<params::ExcludeTenantOwnerParams>();

    vec![
        serde_json::json!({
            "name": "managers.accounts.createSystemAccount",
            "summary": "Create a system account",
            "description": "Creates a system account (gateway manager, guests manager, or system manager). Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "accounts" }],
            "params": [{ "name": "params", "description": "Creation parameters", "required": true, "schema": create_system_account_schema }],
            "result": { "name": "result", "description": "Created or existing account (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.guestRoles.createSystemRoles",
            "summary": "Create system guest roles",
            "description": "Creates all system guest roles (subscriptions, users, account, guest, gateway, system, tenant managers with read/write). Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "guest-roles" }],
            "params": [],
            "result": { "name": "result", "description": "List of guest roles created", "schema": { "type": "array", "items": { "type": "object" } } },
            "errors": [{ "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.createTenant",
            "summary": "Create a tenant",
            "description": "Creates a new tenant with the given owner. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": create_tenant_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.listTenant",
            "summary": "List tenants",
            "description": "Lists tenants with optional filters (name, owner, metadata, tag) and pagination (pageSize, skip).",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": false, "schema": list_tenant_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.deleteTenant",
            "summary": "Delete a tenant",
            "description": "Deletes a tenant by ID. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": delete_tenant_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.includeTenantOwner",
            "summary": "Include a tenant owner",
            "description": "Adds an owner to a tenant. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": include_tenant_owner_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.excludeTenantOwner",
            "summary": "Exclude a tenant owner",
            "description": "Removes an owner from a tenant. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": exclude_tenant_owner_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
