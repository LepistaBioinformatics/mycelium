use super::{account::Account, email::Email, role::Role};

use chrono::{DateTime, Local};
use mycelium_base::dtos::{Children, Parent};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Permissions {
    View = 0,
    Create = 1,
    Update = 2,
    Delete = 3,
}

impl Permissions {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Permissions::View,
            1 => Permissions::Create,
            2 => Permissions::Update,
            3 => Permissions::Delete,
            _ => Permissions::View,
        }
    }
}

impl FromStr for Permissions {
    type Err = ();

    fn from_str(s: &str) -> Result<Permissions, ()> {
        match s {
            "view" => Ok(Permissions::View),
            "create" => Ok(Permissions::Create),
            "update" => Ok(Permissions::Update),
            "delete" => Ok(Permissions::Delete),
            _ => Err(()),
        }
    }
}

impl ToString for Permissions {
    fn to_string(&self) -> String {
        match self {
            Permissions::View => "view".to_string(),
            Permissions::Create => "create".to_string(),
            Permissions::Update => "update".to_string(),
            Permissions::Delete => "delete".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuestRole {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: Option<String>,
    pub role: Parent<Role, Uuid>,
    pub permissions: Vec<Permissions>,
}

impl GuestRole {
    pub fn build_role_url(&self, base_url: String) -> Result<String, ()> {
        match self.role.to_owned() {
            Parent::Id(id) => Ok(format!("{}/{}", base_url, id)),
            Parent::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuestUser {
    pub id: Option<Uuid>,

    pub email: Email,
    pub guest_role: Parent<GuestRole, Uuid>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub accounts: Option<Children<Account, Uuid>>,
}

impl GuestUser {
    pub fn build_role_url(&self, base_url: String) -> Result<String, ()> {
        match self.guest_role.to_owned() {
            Parent::Id(id) => Ok(format!("{}/{}", base_url, id)),
            Parent::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }
}
