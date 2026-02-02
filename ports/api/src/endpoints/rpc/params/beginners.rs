//! JSON-RPC param DTOs for beginners scope (beginners.accounts.*, beginners.guests.*, beginners.meta.*, beginners.profile.*).

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
