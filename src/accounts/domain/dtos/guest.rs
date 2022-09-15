use serde::{Deserialize, Serialize};

use super::application::ApplicationDTO;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PermissionsType {
    View = 0,
    Create = 1,
    Update = 2,
    Delete = 3,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserRoleDTO {
    pub id: String,

    pub name: String,
    pub application: ApplicationDTO,
    pub description: String,
    pub permissions: Vec<PermissionsType>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuestUserDTO {
    pub id: String,

    pub email: String,
    pub role: UserRoleDTO,
}
