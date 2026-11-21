use serde::{Deserialize, Serialize};

/// Who may read the role catalog, from `federation.roleCatalog` (D-10)
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum RoleCatalogMode {
    /// Only a request signed by a declared peer
    #[default]
    Peers,

    /// Anyone, like the self-description
    Public,

    /// Not served
    Off,
}
