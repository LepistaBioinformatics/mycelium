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
    pub id: Option<Uuid>,

    pub name: String,
    pub description: String,
    pub application: ParentEnum<Uuid, ApplicationDTO>,
    pub permissions: Vec<PermissionsType>,
}

impl UserRoleDTO {
    pub fn build_application_url(
        &self,
        base_url: String,
    ) -> Result<String, ()> {
        match self.application.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{}/{}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserDTO {
    pub id: Option<Uuid>,

    pub email: String,
    pub role: ParentEnum<Uuid, UserRoleDTO>,
}

impl GuestUserDTO {
    pub fn build_role_url(&self, base_url: String) -> Result<String, ()> {
        match self.role.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{}/{}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }
}
