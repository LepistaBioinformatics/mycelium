use super::guest_role::Permission;

use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

pub type PermissionedRoles = Vec<(String, Permission)>;

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum RouteType {
    ///
    /// Allow public access to the route
    ///
    Public,
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
