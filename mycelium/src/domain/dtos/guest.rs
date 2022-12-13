use super::{account::AccountDTO, role::RoleDTO};
use agrobase::dtos::enums::ParentEnum;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionsType {
    View = 0,
    Create = 1,
    Update = 2,
    Delete = 3,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestRoleDTO {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: String,
    pub role: ParentEnum<RoleDTO, Uuid>,
    pub permissions: Vec<PermissionsType>,
    pub account: ParentEnum<AccountDTO, Uuid>,
}

impl GuestRoleDTO {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserDTO {
    pub id: Option<Uuid>,

    pub email: String,
    pub role: ParentEnum<GuestRoleDTO, Uuid>,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
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
