use super::{application::ApplicationDTO, enums::ParentEnum};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionsType {
    View = 0,
    Create = 1,
    Update = 2,
    Delete = 3,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRoleDTO {
    pub id: Uuid,

    pub name: String,
    pub description: String,
    pub application: ParentEnum<Uuid, ApplicationDTO>,
    pub permissions: Vec<PermissionsType>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserDTO {
    pub id: Uuid,

    pub email: String,
    pub role: ParentEnum<Uuid, UserRoleDTO>,
}
