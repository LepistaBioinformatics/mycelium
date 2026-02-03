use super::super::schema;
use crate::rpc::{method_names, params};

pub fn methods() -> Vec<serde_json::Value> {
    let create_management_account_schema =
        schema::param_schema_value::<params::CreateManagementAccountParams>();
    let delete_tenant_manager_account_schema = schema::param_schema_value::<
        params::DeleteTenantManagerAccountParams,
    >();
    let create_tenant_meta_schema =
        schema::param_schema_value::<params::CreateTenantMetaParams>();
    let delete_tenant_meta_schema =
        schema::param_schema_value::<params::DeleteTenantMetaParams>();
    let guest_tenant_owner_schema =
        schema::param_schema_value::<params::GuestTenantOwnerParams>();
    let revoke_tenant_owner_schema =
        schema::param_schema_value::<params::RevokeTenantOwnerParams>();
    let update_tenant_name_and_description_schema = schema::param_schema_value::<
        params::UpdateTenantNameAndDescriptionParams,
    >();
    let update_tenant_archiving_status_schema = schema::param_schema_value::<
        params::UpdateTenantArchivingStatusParams,
    >();
    let update_tenant_trashing_status_schema = schema::param_schema_value::<
        params::UpdateTenantTrashingStatusParams,
    >();
    let update_tenant_verifying_status_schema = schema::param_schema_value::<
        params::UpdateTenantVerifyingStatusParams,
    >();

    vec![
        serde_json::json!({
            "name": method_names::TENANT_OWNER_ACCOUNTS_CREATE_MANAGEMENT_ACCOUNT,
            "summary": "Create management account",
            "description": "Creates a tenant-related management account. Requires TenantOwner privileges.",
            "tags": [{ "name": "tenantOwner" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": create_management_account_schema }],
            "result": { "name": "result", "description": "Created or existing account (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_ACCOUNTS_DELETE_TENANT_MANAGER_ACCOUNT,
            "summary": "Delete tenant manager account",
            "description": "Soft deletes the tenant manager account. Requires TenantOwner privileges.",
            "tags": [{ "name": "tenantOwner" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": delete_tenant_manager_account_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_META_CREATE,
            "summary": "Create tenant meta",
            "description": "Registers tenant metadata (key/value). Key e.g. federal_revenue_register, locale, legal_name.",
            "tags": [{ "name": "tenantOwner" }, { "name": "meta" }],
            "params": [{ "name": "params", "required": true, "schema": create_tenant_meta_schema }],
            "result": { "name": "result", "description": "Created meta (CreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_META_DELETE,
            "summary": "Delete tenant meta",
            "description": "Deletes tenant metadata by key.",
            "tags": [{ "name": "tenantOwner" }, { "name": "meta" }],
            "params": [{ "name": "params", "required": true, "schema": delete_tenant_meta_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_OWNER_GUEST,
            "summary": "Guest tenant owner",
            "description": "Adds a user (by email) as tenant owner.",
            "tags": [{ "name": "tenantOwner" }, { "name": "owner" }],
            "params": [{ "name": "params", "required": true, "schema": guest_tenant_owner_schema }],
            "result": { "name": "result", "description": "Created TenantOwnerConnection (CreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_OWNER_REVOKE,
            "summary": "Revoke tenant owner",
            "description": "Revokes a user (by email) from tenant ownership.",
            "tags": [{ "name": "tenantOwner" }, { "name": "owner" }],
            "params": [{ "name": "params", "required": true, "schema": revoke_tenant_owner_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_TENANT_UPDATE_AND_DESCRIPTION,
            "summary": "Update tenant name and description",
            "description": "Updates the name and/or description of a tenant.",
            "tags": [{ "name": "tenantOwner" }, { "name": "tenant" }],
            "params": [{ "name": "params", "required": true, "schema": update_tenant_name_and_description_schema }],
            "result": { "name": "result", "description": "Updated tenant (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_TENANT_UPDATE_ARCHIVING_STATUS,
            "summary": "Update tenant archiving status",
            "description": "Sets the tenant as archived.",
            "tags": [{ "name": "tenantOwner" }, { "name": "tenant" }],
            "params": [{ "name": "params", "required": true, "schema": update_tenant_archiving_status_schema }],
            "result": { "name": "result", "description": "Updated tenant (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_TENANT_UPDATE_TRASHING_STATUS,
            "summary": "Update tenant trashing status",
            "description": "Sets the tenant as trashed.",
            "tags": [{ "name": "tenantOwner" }, { "name": "tenant" }],
            "params": [{ "name": "params", "required": true, "schema": update_tenant_trashing_status_schema }],
            "result": { "name": "result", "description": "Updated tenant (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_OWNER_TENANT_UPDATE_VERIFYING_STATUS,
            "summary": "Update tenant verifying status",
            "description": "Sets the tenant as verified.",
            "tags": [{ "name": "tenantOwner" }, { "name": "tenant" }],
            "params": [{ "name": "params", "required": true, "schema": update_tenant_verifying_status_schema }],
            "result": { "name": "result", "description": "Updated tenant (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
