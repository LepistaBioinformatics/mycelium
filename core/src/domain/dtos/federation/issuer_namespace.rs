use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The tenants and accounts an issuer claims as its own (I-1)
///
/// The consumer checks these ids structurally against its own ids and other
/// peers' claims; it never trusts the grants themselves for ownership.
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuerNamespace {
    pub tenants: Vec<Uuid>,
    pub accounts: Vec<Uuid>,
}

impl IssuerNamespace {
    /// Every claimed id, tenants first
    pub fn ids(&self) -> Vec<Uuid> {
        self.tenants.iter().chain(&self.accounts).copied().collect()
    }
}
