use mycelium_base::dtos::Children;
use serde::{Deserialize, Serialize};
use slugify::slugify;
use std::str::FromStr;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Read = 0,
    Write = 1,
    ReadWrite = 2,
}

impl Permission {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Permission::Read,
            1 => Permission::Write,
            2 => Permission::ReadWrite,
            _ => Permission::Read,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Permission::Read => 0,
            Permission::Write => 1,
            Permission::ReadWrite => 2,
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Permission, ()> {
        match s {
            "read" => Ok(Permission::Read),
            "write" => Ok(Permission::Write),
            "read-write" => Ok(Permission::ReadWrite),
            _ => {
                error!("Invalid permission: {}", s);
                Ok(Permission::Read)
            }
        }
    }
}

impl ToString for Permission {
    fn to_string(&self) -> String {
        match self {
            Permission::Read => "read".to_string(),
            Permission::Write => "write".to_string(),
            Permission::ReadWrite => "read-write".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuestRole {
    pub id: Option<Uuid>,

    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub permission: Permission,

    /// Children roles represents guest roles that are children of the current
    /// role, and should be used to determine the allowed roles for the role
    /// owner guest other users.
    pub children: Option<Children<GuestRole, Uuid>>,
}

impl GuestRole {
    pub fn new(
        id: Option<Uuid>,
        name: String,
        description: Option<String>,
        permission: Permission,
        children: Option<Children<GuestRole, Uuid>>,
    ) -> Self {
        GuestRole {
            id,
            name: name.to_owned(),
            slug: slugify!(&name),
            description,
            permission,
            children,
        }
    }
}
