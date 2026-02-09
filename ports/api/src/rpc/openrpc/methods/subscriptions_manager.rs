use super::super::schema;
use crate::rpc::{method_names, params::subscriptions_manager};

pub fn methods() -> Vec<serde_json::Value> {
    let create_subscription_account_schema = schema::param_schema_value::<
        subscriptions_manager::CreateSubscriptionAccountParams,
    >();
    let create_role_associated_schema = schema::param_schema_value::<
        subscriptions_manager::CreateRoleAssociatedAccountParams,
    >();
    let list_accounts_by_type_schema = schema::param_schema_value::<
        subscriptions_manager::ListAccountsByTypeParams,
    >();
    let get_account_details_schema = schema::param_schema_value::<
        subscriptions_manager::GetAccountDetailsParams,
    >();
    let update_account_name_and_flags_schema = schema::param_schema_value::<
        subscriptions_manager::UpdateAccountNameAndFlagsParams,
    >();
    let propagate_subscription_account_schema = schema::param_schema_value::<
        subscriptions_manager::PropagateSubscriptionAccountParams,
    >();
    let list_licensed_accounts_of_email_schema = schema::param_schema_value::<
        subscriptions_manager::ListLicensedAccountsOfEmailParams,
    >();
    let guest_user_to_subscription_account_schema = schema::param_schema_value::<
        subscriptions_manager::GuestUserToSubscriptionAccountParams,
    >();
    let update_flags_from_subscription_account_schema =
        schema::param_schema_value::<
            subscriptions_manager::UpdateFlagsFromSubscriptionAccountParams,
        >();
    let revoke_user_guest_schema = schema::param_schema_value::<
        subscriptions_manager::RevokeUserGuestToSubscriptionAccountParams,
    >();
    let list_guest_on_subscription_account_schema = schema::param_schema_value::<
        subscriptions_manager::ListGuestOnSubscriptionAccountParams,
    >();
    let list_guest_roles_schema = schema::param_schema_value::<
        subscriptions_manager::SubscriptionsManagerListGuestRolesParams,
    >();
    let fetch_guest_role_details_schema = schema::param_schema_value::<
        subscriptions_manager::SubscriptionsManagerFetchGuestRoleDetailsParams,
    >();
    let register_tag_schema = schema::param_schema_value::<
        subscriptions_manager::RegisterTagParams,
    >();
    let update_tag_schema =
        schema::param_schema_value::<subscriptions_manager::UpdateTagParams>();
    let delete_tag_schema =
        schema::param_schema_value::<subscriptions_manager::DeleteTagParams>();

    vec![
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_CREATE_SUBSCRIPTION_ACCOUNT,
            "summary": "Create subscription account",
            "description": "Creates a subscription account for the tenant. Requires SubscriptionsManager privileges.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": create_subscription_account_schema }],
            "result": { "name": "result", "description": "Created account", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_CREATE_ROLE_ASSOCIATED_ACCOUNT,
            "summary": "Create role associated account",
            "description": "Creates or returns existing role-associated account. Requires SubscriptionsManager privileges.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": create_role_associated_schema }],
            "result": { "name": "result", "description": "Created or existing account (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_LIST,
            "summary": "List accounts by type",
            "description": "Lists accounts filtered by type (Staff, Manager, User, Subscription, TenantManager, ActorAssociated, RoleAssociated), status and pagination.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": false, "schema": list_accounts_by_type_schema }],
            "result": { "name": "result", "description": "Paginated list or array (FetchManyResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_GET,
            "summary": "Get account details",
            "description": "Returns a single subscription account by ID. Optional tenant ID to scope.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": get_account_details_schema }],
            "result": { "name": "result", "description": "Account or null (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_UPDATE_NAME_AND_FLAGS,
            "summary": "Update account name and flags",
            "description": "Updates name and flags (isActive, isChecked, isArchived, isSystemAccount) of a subscription account.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": update_account_name_and_flags_schema }],
            "result": { "name": "result", "description": "Updated account (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_PROPAGATE_SUBSCRIPTION_ACCOUNT,
            "summary": "Propagate subscription account",
            "description": "Propagates a subscription account. Requires SubscriptionsManager privileges.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "accounts" }],
            "params": [{ "name": "params", "required": true, "schema": propagate_subscription_account_schema }],
            "result": { "name": "result", "description": "Propagated account", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUESTS_LIST_LICENSED_ACCOUNTS_OF_EMAIL,
            "summary": "List licensed accounts of email",
            "description": "Lists subscription accounts for which the given email is a guest. Optional roles filter.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": list_licensed_accounts_of_email_schema }],
            "result": { "name": "result", "description": "List of licensed resources (FetchManyResponseKind)", "schema": { "type": "array" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUESTS_GUEST_USER_TO_SUBSCRIPTION_ACCOUNT,
            "summary": "Guest user to subscription account",
            "description": "Adds a guest user (by email) to a subscription account under the given role.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": guest_user_to_subscription_account_schema }],
            "result": { "name": "result", "description": "Created or existing guest user (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUESTS_UPDATE_FLAGS_FROM_SUBSCRIPTION_ACCOUNT,
            "summary": "Update flags from subscription account",
            "description": "Updates permit/deny flags for a guest user on a subscription account.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": update_flags_from_subscription_account_schema }],
            "result": { "name": "result", "description": "Updated guest user (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUESTS_REVOKE_USER_GUEST_TO_SUBSCRIPTION_ACCOUNT,
            "summary": "Revoke user guest to subscription account",
            "description": "Revokes a guest user (by email) from a subscription account role.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": revoke_user_guest_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUESTS_LIST_GUEST_ON_SUBSCRIPTION_ACCOUNT,
            "summary": "List guest on subscription account",
            "description": "Lists guest users on a subscription account with optional pagination.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guests" }],
            "params": [{ "name": "params", "required": true, "schema": list_guest_on_subscription_account_schema }],
            "result": { "name": "result", "description": "Paginated list of guest users (FetchManyResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUEST_ROLES_LIST,
            "summary": "List guest roles",
            "description": "Lists guest roles with optional filters and pagination. Optional tenant ID to scope.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": false, "schema": list_guest_roles_schema }],
            "result": { "name": "result", "description": "List of guest roles (FetchManyResponseKind)", "schema": { "type": "array" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_GUEST_ROLES_GET,
            "summary": "Fetch guest role details",
            "description": "Returns details for a guest role by ID. Optional tenant ID to scope.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "guestRoles" }],
            "params": [{ "name": "params", "required": true, "schema": fetch_guest_role_details_schema }],
            "result": { "name": "result", "description": "Guest role or null (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_TAGS_CREATE,
            "summary": "Register tag",
            "description": "Registers a tag on a subscription account.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "tags" }],
            "params": [{ "name": "params", "required": true, "schema": register_tag_schema }],
            "result": { "name": "result", "description": "Created or existing tag (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_TAGS_UPDATE,
            "summary": "Update tag",
            "description": "Updates a tag (value, meta) on a subscription account.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "tags" }],
            "params": [{ "name": "params", "required": true, "schema": update_tag_schema }],
            "result": { "name": "result", "description": "Updated tag (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": method_names::SUBSCRIPTIONS_MANAGER_TAGS_DELETE,
            "summary": "Delete tag",
            "description": "Deletes a tag from a subscription account.",
            "tags": [{ "name": "subscriptionsManager" }, { "name": "tags" }],
            "params": [{ "name": "params", "required": true, "schema": delete_tag_schema }],
            "result": { "name": "result", "description": "null on success (DeletionResponseKind)", "schema": { "type": "null" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
