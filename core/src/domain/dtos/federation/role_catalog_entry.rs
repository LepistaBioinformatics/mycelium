use crate::domain::dtos::guest_role::Permission;

use serde::{Deserialize, Serialize};

/// One guest role as an instance publishes it to its peers (D-10, FED-55)
///
/// Carries no tenant or account id: which tenants use a role is not
/// published.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleCatalogEntry {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub permission: Permission,
    pub system: bool,

    /// Slugs of the child roles
    pub children: Vec<String>,
}
