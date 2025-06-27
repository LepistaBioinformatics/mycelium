use chrono::{DateTime, Local};
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
}

impl Permission {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Permission::Read,
            1 => Permission::Write,
            _ => Permission::Read,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Permission::Read => 0,
            Permission::Write => 1,
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Permission, ()> {
        match s {
            "read" => Ok(Permission::Read),
            "write" => Ok(Permission::Write),
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

    /// The date and time the role was created
    pub created: DateTime<Local>,

    /// The date and time the role was last updated
    pub updated: Option<DateTime<Local>>,

    /// If it is a system role
    ///
    /// System roles represents standard core actors of the Mycelium API
    /// Gateway, defined in `SystemActor`
    ///
    pub system: bool,

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
        system: bool,
    ) -> Self {
        GuestRole {
            id,
            name: name.to_owned(),
            slug: slugify!(&name),
            description,
            permission,
            children,
            system,
            created: Local::now(),
            updated: None,
        }
    }
}
