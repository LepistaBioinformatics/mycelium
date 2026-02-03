//! JSON-RPC param DTOs for account manager scope (accountManager.guests.*, accountManager.guestRoles.*).

use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Guests (guest to children account)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestToChildrenAccountParams {
    #[schemars(
        description = "Tenant ID (must match x-mycelium-tenant-id context)"
    )]
    pub tenant_id: Uuid,
    #[schemars(description = "Target account ID")]
    pub account_id: Uuid,
    #[schemars(description = "Guest role ID (child role to assign)")]
    pub role_id: Uuid,
    #[schemars(description = "Guest user email")]
    pub email: String,
}

// ---------------------------------------------------------------------------
// Guest roles (list, fetch details)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestRolesParams {
    #[schemars(description = "Optional tenant ID to scope the list")]
    pub tenant_id: Option<Uuid>,
    #[schemars(description = "Filter by guest role name")]
    pub name: Option<String>,
    #[schemars(description = "Filter by guest role slug")]
    pub slug: Option<String>,
    #[schemars(description = "Filter by system role flag")]
    pub system: Option<bool>,
    #[schemars(description = "Page size for pagination")]
    pub page_size: Option<i32>,
    #[schemars(description = "Number of records to skip")]
    pub skip: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FetchGuestRoleDetailsParams {
    #[schemars(description = "Guest role ID")]
    pub id: Uuid,
    #[schemars(description = "Optional tenant ID to scope the request")]
    pub tenant_id: Option<Uuid>,
}
