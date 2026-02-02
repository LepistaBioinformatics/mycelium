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
    let fetch_tenant_public_info_schema =
        schema::param_schema_value::<params::FetchTenantPublicInfoParams>();
    let create_connection_string_schema =
        schema::param_schema_value::<params::CreateConnectionStringParams>();
    let create_default_user_schema =
        schema::param_schema_value::<params::CreateDefaultUserParams>();
    let check_token_and_activate_user_schema =
        schema::param_schema_value::<params::CheckTokenAndActivateUserParams>();
    let start_password_redefinition_schema =
        schema::param_schema_value::<params::StartPasswordRedefinitionParams>();
    let check_token_and_reset_password_schema = schema::param_schema_value::<
        params::CheckTokenAndResetPasswordParams,
    >();
    let check_email_password_validity_schema = schema::param_schema_value::<
        params::CheckEmailPasswordValidityParams,
    >();
    let totp_start_activation_schema =
        schema::param_schema_value::<params::TotpStartActivationParams>();
    let totp_finish_activation_schema =
        schema::param_schema_value::<params::TotpFinishActivationParams>();
    let totp_check_token_schema =
        schema::param_schema_value::<params::TotpCheckTokenParams>();
    let totp_disable_schema =
        schema::param_schema_value::<params::TotpDisableParams>();

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
        serde_json::json!({
            "name": "beginners.tenants.fetchTenantPublicInfo",
            "summary": "Fetch tenant public info",
            "description": "Returns public info for a tenant. Profile must have tenant license or ownership.",
            "tags": [{ "name": "beginners" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": fetch_tenant_public_info_schema }],
            "result": { "name": "result", "description": "Tenant or not found (FetchResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.tokens.createConnectionString",
            "summary": "Create connection string",
            "description": "Creates a connection string for the user account. Optional tenant_id, service_account_id, roles to scope the token.",
            "tags": [{ "name": "beginners" }, { "name": "tokens" }],
            "params": [{ "name": "params", "required": true, "schema": create_connection_string_schema }],
            "result": { "name": "result", "description": "Object with connectionString", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.tokens.listMyConnectionStrings",
            "summary": "List my connection strings",
            "description": "Lists all connection strings for the current user.",
            "tags": [{ "name": "beginners" }, { "name": "tokens" }],
            "params": [],
            "result": { "name": "result", "description": "List of PublicConnectionStringInfo", "schema": { "type": "array" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.createDefaultUser",
            "summary": "Create default user",
            "description": "Registers a new user. Optional Bearer token to register with provider; otherwise password required.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": create_default_user_schema }],
            "result": { "name": "result", "description": "Success message", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32600, "message": "Invalid request" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.checkTokenAndActivateUser",
            "summary": "Check token and activate user",
            "description": "Validates activation token and activates the user.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": check_token_and_activate_user_schema }],
            "result": { "name": "result", "description": "Activated user (User)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.startPasswordRedefinition",
            "summary": "Start password redefinition",
            "description": "Starts the password reset process for the given email.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": start_password_redefinition_schema }],
            "result": { "name": "result", "description": "true on success", "schema": { "type": "boolean" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.checkTokenAndResetPassword",
            "summary": "Check token and reset password",
            "description": "Validates reset token and sets new password.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": check_token_and_reset_password_schema }],
            "result": { "name": "result", "description": "true on success", "schema": { "type": "boolean" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.checkEmailPasswordValidity",
            "summary": "Check email and password validity",
            "description": "Validates email and password. Returns { valid, user? }. Does not issue JWT.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": check_email_password_validity_schema }],
            "result": { "name": "result", "description": "valid and optional user", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.totpStartActivation",
            "summary": "TOTP start activation",
            "description": "Starts TOTP activation for the user. Optional qrCode to get QR code URL.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": totp_start_activation_schema }],
            "result": { "name": "result", "description": "totpUrl and totpSecret", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.totpFinishActivation",
            "summary": "TOTP finish activation",
            "description": "Finishes TOTP activation with the token from the authenticator app.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": totp_finish_activation_schema }],
            "result": { "name": "result", "description": "finished: true", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.totpCheckToken",
            "summary": "TOTP check token",
            "description": "Validates TOTP token and returns the user.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": totp_check_token_schema }],
            "result": { "name": "result", "description": "User", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "beginners.users.totpDisable",
            "summary": "TOTP disable",
            "description": "Disables TOTP for the user.",
            "tags": [{ "name": "beginners" }, { "name": "users" }],
            "params": [{ "name": "params", "required": true, "schema": totp_disable_schema }],
            "result": { "name": "result", "description": "Empty object on success", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
