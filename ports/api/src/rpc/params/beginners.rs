use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Accounts (create default account, update own account name, delete my account)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultAccountParams {
    #[schemars(description = "Account name")]
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOwnAccountNameParams {
    #[schemars(
        description = "Account ID (must match the authenticated user's account)"
    )]
    pub account_id: Uuid,
    #[schemars(description = "New account name")]
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMyAccountParams {
    #[schemars(
        description = "Account ID (must match the authenticated user's account)"
    )]
    pub account_id: Uuid,
}

// ---------------------------------------------------------------------------
// Guests (accept invitation)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptInvitationParams {
    #[schemars(description = "Account ID the user is invited to")]
    pub account_id: Uuid,
    #[schemars(description = "Guest role unique name")]
    pub guest_role_name: String,
    #[schemars(description = "Permission to grant: 0 = Read, 1 = Write")]
    pub permission: i32,
}

// ---------------------------------------------------------------------------
// Meta (account metadata: create, update, delete)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountMetaParams {
    #[schemars(
        description = "Meta key (e.g. phone_number, telegram_user, locale, custom:key)"
    )]
    pub key: String,
    #[schemars(description = "Meta value")]
    pub value: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountMetaParams {
    #[schemars(description = "Meta key")]
    pub key: String,
    #[schemars(description = "Meta value")]
    pub value: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAccountMetaParams {
    #[schemars(description = "Meta key")]
    pub key: String,
}

// ---------------------------------------------------------------------------
// Profile (fetch my profile)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FetchMyProfileParams {
    #[schemars(
        description = "If true (default), expand licensed resources and tenants ownership as URL strings"
    )]
    pub with_url: Option<bool>,
}

// ---------------------------------------------------------------------------
// Tenants (fetch tenant public info)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FetchTenantPublicInfoParams {
    #[schemars(description = "Tenant ID")]
    pub tenant_id: Uuid,
}

// ---------------------------------------------------------------------------
// Tokens (create connection string, list my connection strings)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleParam {
    pub name: String,
    #[schemars(description = "0 = Read, 1 = Write")]
    pub permission: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateConnectionStringParams {
    #[schemars(description = "Token name")]
    pub name: String,
    #[schemars(description = "Expiration time in seconds")]
    pub expiration: i64,
    #[schemars(description = "Optional tenant ID to scope the token")]
    pub tenant_id: Option<Uuid>,
    #[schemars(description = "Optional service account ID to scope the token")]
    pub service_account_id: Option<Uuid>,
    #[schemars(description = "Optional roles to scope the token")]
    pub roles: Option<Vec<RoleParam>>,
}

// ---------------------------------------------------------------------------
// Users (create default user, token activation, password reset, TOTP)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultUserParams {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenAndActivateUserParams {
    pub token: String,
    pub email: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartPasswordRedefinitionParams {
    pub email: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenAndResetPasswordParams {
    pub token: String,
    pub email: String,
    pub new_password: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckEmailPasswordValidityParams {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpStartActivationParams {
    pub email: String,
    #[schemars(description = "If true, return QR code URL")]
    pub qr_code: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpFinishActivationParams {
    pub email: String,
    pub token: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpCheckTokenParams {
    pub email: String,
    pub token: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpDisableParams {
    pub email: String,
    pub token: String,
}
