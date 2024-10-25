use super::role::Role;

use mycelium_base::dtos::{Children, Parent};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Permissions {
    Read = 0,
    Write = 1,
}

impl Permissions {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Permissions::Read,
            1 => Permissions::Write,
            _ => Permissions::Read,
        }
    }
}

impl FromStr for Permissions {
    type Err = ();

    fn from_str(s: &str) -> Result<Permissions, ()> {
        match s {
            "read" => Ok(Permissions::Read),
            "write" => Ok(Permissions::Write),
            _ => Err(()),
        }
    }
}

impl ToString for Permissions {
    fn to_string(&self) -> String {
        match self {
            Permissions::Read => "read".to_string(),
            Permissions::Write => "write".to_string(),
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

    /// Children roles represents guest roles that are children of the current
    /// role, and should be used to determine the allowed roles for the role
    /// owner guest other users.
    pub children: Option<Children<GuestRole, Uuid>>,
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
