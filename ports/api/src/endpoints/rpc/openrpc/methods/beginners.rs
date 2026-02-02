//! OpenRPC method descriptors for beginners scope (beginners.accounts.*, beginners.guests.*, beginners.meta.*, beginners.profile.*).

use super::super::schema;
use crate::endpoints::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let create_default_account_schema =
        schema::param_schema_value::<params::CreateDefaultAccountParams>();
    let update_own_account_name_schema =
        schema::param_schema_value::<params::UpdateOwnAccountNameParams>();
    let delete_my_account_schema =
        schema::param_schema_value::<params::DeleteMyAccountParams>();
    let accept_invitation_schema =
        schema::param_schema_value::<params::AcceptInvitationParams>();
    let create_account_meta_schema =
        schema::param_schema_value::<params::CreateAccountMetaParams>();
    let update_account_meta_schema =
        schema::param_schema_value::<params::UpdateAccountMetaParams>();
    let delete_account_meta_schema =
        schema::param_schema_value::<params::DeleteAccountMetaParams>();
    let fetch_my_profile_schema =
        schema::param_schema_value::<params::FetchMyProfileParams>();

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
        serde_json::json!({
            "name": "beginners.guests.acceptInvitation",
            "summary": "Accept invitation",
            "description": "Accepts an invitation to join an account as a guest. License must match account_id, guest_role_name and permission.",
            "tags": [{ "name": "beginners" }, { "name": "guests" }],
            "params": [{ "name": "params", "description": "Account ID, guest role name, permission (0=Read, 1=Write)", "required": true, "schema": accept_invitation_schema }],
            "result": { "name": "result", "description": "Updated (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.meta.createAccountMeta",
            "summary": "Create account metadata",
            "description": "Registers a metadata key-value for the current account (e.g. phone_number, telegram_user, locale, custom:key).",
            "tags": [{ "name": "beginners" }, { "name": "meta" }],
            "params": [{ "name": "params", "description": "Key and value", "required": true, "schema": create_account_meta_schema }],
            "result": { "name": "result", "description": "Created (CreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.meta.updateAccountMeta",
            "summary": "Update account metadata",
            "description": "Updates a metadata key-value for the current account.",
            "tags": [{ "name": "beginners" }, { "name": "meta" }],
            "params": [{ "name": "params", "description": "Key and value", "required": true, "schema": update_account_meta_schema }],
            "result": { "name": "result", "description": "Updated (UpdatingResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.meta.deleteAccountMeta",
            "summary": "Delete account metadata",
            "description": "Deletes a metadata key for the current account.",
            "tags": [{ "name": "beginners" }, { "name": "meta" }],
            "params": [{ "name": "params", "description": "Meta key", "required": true, "schema": delete_account_meta_schema }],
            "result": { "name": "result", "description": "Deletion result (DeletionResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.profile.fetchMyProfile",
            "summary": "Fetch my profile",
            "description": "Returns the current user's Mycelium profile. Optionally expand licensed resources and tenants ownership as URL strings (withUrl, default true).",
            "tags": [{ "name": "beginners" }, { "name": "profile" }],
            "params": [{ "name": "params", "description": "Optional withUrl (default true)", "required": false, "schema": fetch_my_profile_schema }],
            "result": { "name": "result", "description": "Profile", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
