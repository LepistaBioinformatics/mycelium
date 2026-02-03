use super::super::schema;
use crate::rpc::{method_names, params};

pub fn methods() -> Vec<serde_json::Value> {
    let create_subscription_manager_account_schema = schema::param_schema_value::<
        params::CreateSubscriptionManagerAccountParams,
    >();
    let delete_subscription_account_schema =
        schema::param_schema_value::<params::DeleteSubscriptionAccountParams>();
    let guest_user_to_subscription_manager_schema = schema::param_schema_value::<
        params::GuestUserToSubscriptionManagerAccountParams,
    >();
    let revoke_user_guest_schema = schema::param_schema_value::<
        params::RevokeUserGuestToSubscriptionManagerAccountParams,
    >();
    let register_tag_schema =
        schema::param_schema_value::<params::TenantManagerRegisterTagParams>();
    let update_tag_schema =
        schema::param_schema_value::<params::TenantManagerUpdateTagParams>();
    let delete_tag_schema =
        schema::param_schema_value::<params::TenantManagerDeleteTagParams>();
    let get_tenant_details_schema =
        schema::param_schema_value::<params::GetTenantDetailsParams>();

    vec![
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_ACCOUNTS_CREATE_SUBSCRIPTION_MANAGER_ACCOUNT,
            "summary": "Create subscription manager account",
            "description": "Creates a tenant-related subscription manager account. Requires TenantManager privileges.",
            "tags": [{ "name": "tenantManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": create_subscription_manager_account_schema }],
            "result": { "name": "result", "description": "Created or existing account (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_ACCOUNTS_DELETE_SUBSCRIPTION_ACCOUNT,
            "summary": "Delete subscription account",
            "description": "Deletes a subscription account. Requires TenantManager privileges.",
            "tags": [{ "name": "tenantManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": delete_subscription_account_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_GUESTS_GUEST_USER_TO_SUBSCRIPTION_MANAGER_ACCOUNT,
            "summary": "Guest user to subscription manager account",
            "description": "Adds a guest user (by email) to a subscription manager account with the given permission (0 = Read, 1 = Write).",
            "tags": [{ "name": "tenantManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": guest_user_to_subscription_manager_schema }],
            "result": { "name": "result", "description": "Created or existing guest user (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_GUESTS_REVOKE_USER_GUEST_TO_SUBSCRIPTION_MANAGER_ACCOUNT,
            "summary": "Revoke user guest to subscription manager account",
            "description": "Revokes a guest user (by email) from a subscription manager account role.",
            "tags": [{ "name": "tenantManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": revoke_user_guest_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_TAGS_CREATE,
            "summary": "Register tenant tag",
            "description": "Registers a tag for the tenant (tenant-scoped, no account).",
            "tags": [{ "name": "tenantManager" }, { "name": "tags" }],
            "params": [{ "name": "params", "required": true, "schema": register_tag_schema }],
            "result": { "name": "result", "description": "Created or existing tag (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_TAGS_UPDATE,
            "summary": "Update tenant tag",
            "description": "Updates a tenant tag (value, meta).",
            "tags": [{ "name": "tenantManager" }, { "name": "tags" }],
            "params": [{ "name": "params", "required": true, "schema": update_tag_schema }],
            "result": { "name": "result", "description": "Updated tag (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_TAGS_DELETE,
            "summary": "Delete tenant tag",
            "description": "Deletes a tenant tag.",
            "tags": [{ "name": "tenantManager" }, { "name": "tags" }],
            "params": [{ "name": "params", "required": true, "schema": delete_tag_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::TENANT_MANAGER_TENANT_GET,
            "summary": "Get tenant details",
            "description": "Returns details for a tenant by ID. Requires TenantManager privileges on the tenant.",
            "tags": [{ "name": "tenantManager" }, { "name": "tenant" }],
            "params": [{ "name": "params", "required": true, "schema": get_tenant_details_schema }],
            "result": { "name": "result", "description": "Tenant or null (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
