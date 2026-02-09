use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Guest roles (create, list, delete, update, insert/remove child)
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGuestRoleParams {
    #[schemars(description = "Guest role name")]
    pub name: String,
    #[schemars(description = "Guest role description")]
    pub description: String,
    #[schemars(description = "Permission: 0 = Read, 1 = Write")]
    pub permission: Option<i32>,
    #[schemars(description = "Whether the role is a system role")]
    pub system: bool,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestRolesParams {
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
pub struct DeleteGuestRoleParams {
    #[schemars(description = "Guest role ID")]
    pub guest_role_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuestRoleNameAndDescriptionParams {
    #[schemars(description = "Guest role ID")]
    pub guest_role_id: Uuid,
    #[schemars(description = "New name")]
    pub name: Option<String>,
    #[schemars(description = "New description")]
    pub description: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuestRolePermissionParams {
    #[schemars(description = "Guest role ID")]
    pub guest_role_id: Uuid,
    #[schemars(description = "Permission: 0 = Read, 1 = Write")]
    pub permission: i32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InsertRoleChildParams {
    #[schemars(description = "Parent guest role ID")]
    pub guest_role_id: Uuid,
    #[schemars(description = "Child guest role ID")]
    pub child_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRoleChildParams {
    #[schemars(description = "Parent guest role ID")]
    pub guest_role_id: Uuid,
    #[schemars(description = "Child guest role ID")]
    pub child_id: Uuid,
}
