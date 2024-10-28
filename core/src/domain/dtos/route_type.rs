use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RouteType {
    Public,
    Protected,
    RoleProtected { roles: Vec<String> },
}
