use super::{account::Account, email::Email, role::Role};

use chrono::{DateTime, Local};
use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PermissionsType {
    View = 0,
    Create = 1,
    Update = 2,
    Delete = 3,
}

impl PermissionsType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => PermissionsType::View,
            1 => PermissionsType::Create,
            2 => PermissionsType::Update,
            3 => PermissionsType::Delete,
            _ => PermissionsType::View,
        }
    }
}

impl FromStr for PermissionsType {
    type Err = ();

    fn from_str(s: &str) -> Result<PermissionsType, ()> {
        match s {
            "view" => Ok(PermissionsType::View),
            "create" => Ok(PermissionsType::Create),
            "update" => Ok(PermissionsType::Update),
            "delete" => Ok(PermissionsType::Delete),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestRole {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: Option<String>,
    pub role: ParentEnum<Role, Uuid>,
    pub permissions: Vec<PermissionsType>,
}

impl GuestRole {
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestUser {
    pub id: Option<Uuid>,

    pub email: Email,
    pub guest_role: ParentEnum<GuestRole, Uuid>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub accounts: Option<ChildrenEnum<Account, Uuid>>,
}

impl GuestUser {
    pub fn build_role_url(&self, base_url: String) -> Result<String, ()> {
        match self.guest_role.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{}/{}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }
}
