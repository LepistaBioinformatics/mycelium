use super::guest_role::Permission;

use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

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
    ProtectedByRoles { roles: Vec<String> },
    ///
    /// Protect the route with the user profile filtered by roles and
    /// permissions
    ///
    #[serde(rename_all = "camelCase")]
    ProtectedByPermissionedRoles {
        permissioned_roles: Vec<(String, Permission)>,
    },
    ///
    /// Protect the route with service token associated to a specific role list
    ///
    #[serde(rename_all = "camelCase")]
    ProtectedByServiceTokenWithRole { roles: Vec<String> },
    ///
    /// Protect the route with service token associated to a specific role and
    /// permissions
    ///
    #[serde(rename_all = "camelCase")]
    ProtectedByServiceTokenWithPermissionedRoles {
        permissioned_roles: Vec<(String, Permission)>,
    },
}

impl ToString for SecurityGroup {
    fn to_string(&self) -> String {
        match self {
            SecurityGroup::Public => "public".to_string(),
            SecurityGroup::Authenticated => "authenticated".to_string(),
            SecurityGroup::Protected => "protected".to_string(),
            SecurityGroup::ProtectedByRoles { roles } => {
                format!("protected_by_roles({})", roles.join(", "))
            }
            SecurityGroup::ProtectedByPermissionedRoles {
                permissioned_roles,
            } => {
                format!(
                    "protected_by_permissioned_roles({})",
                    permissioned_roles
                        .iter()
                        .map(|(role, permission)| format!(
                            "{}: {}",
                            role,
                            permission.to_i32()
                        ))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            SecurityGroup::ProtectedByServiceTokenWithRole { roles } => {
                format!(
                    "protected_by_service_token_with_role({})",
                    roles.join(", ")
                )
            }
            SecurityGroup::ProtectedByServiceTokenWithPermissionedRoles {
                permissioned_roles,
            } => {
                format!(
                    "protected_by_service_token_with_permissioned_roles({})",
                    permissioned_roles
                        .iter()
                        .map(|(role, permission)| format!(
                            "{}: {}",
                            role,
                            permission.to_i32()
                        ))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }
}
