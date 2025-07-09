use super::guest_role::Permission;

use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(
    Debug,
    Default,
    Clone,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ToSchema,
    ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct PermissionedRole {
    pub slug: String,

    #[serde(default = "default_permission")]
    pub permission: Option<Permission>,
}

fn default_permission() -> Option<Permission> {
    Some(Permission::Read)
}

impl ToString for PermissionedRole {
    fn to_string(&self) -> String {
        format!(
            "{}: {}",
            self.slug,
            self.permission.clone().unwrap_or_default().to_string()
        )
    }
}

pub type PermissionedRoles = Vec<(String, Permission)>;

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum SecurityGroup {
    ///
    /// Allow public access to the route
    ///
    Public,
    ///
    /// Users should be only authenticated with a valid API token
    ///
    Authenticated,
    ///
    /// Protect the route with the full user profile
    ///
    Protected,
    ///
    /// Protect the route with the user profile filtered by roles
    ///
    #[serde(rename_all = "camelCase")]
    ProtectedByRoles { roles: Vec<PermissionedRole> },
}

impl ToString for SecurityGroup {
    fn to_string(&self) -> String {
        match self {
            SecurityGroup::Public => "public".to_string(),
            SecurityGroup::Authenticated => "authenticated".to_string(),
            SecurityGroup::Protected => "protected".to_string(),
            SecurityGroup::ProtectedByRoles { roles } => {
                format!(
                    "protected_by_roles({})",
                    roles
                        .iter()
                        .map(|permissioned_role| format!(
                            "{}: {}",
                            permissioned_role.slug,
                            permissioned_role
                                .permission
                                .as_ref()
                                .cloned()
                                .unwrap_or_default()
                                .to_i32()
                        ))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }
}
