use super::{FederatedGrants, IssuerNamespace};

use serde::{Deserialize, Serialize};

/// The payload of a `profile.v1` assertion (design §4)
///
/// Carries grants only: no instance-scoped flag (staff, manager, account,
/// status) exists in it, so a peer cannot influence them (FED-28).
///
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAssertion {
    pub grants: FederatedGrants,
    pub issuer_namespace: IssuerNamespace,
}

impl ProfileAssertion {
    /// The assertion for these grants, claiming exactly their ids
    pub fn from_grants(grants: FederatedGrants) -> Self {
        let issuer_namespace = grants.namespace();
        Self {
            grants,
            issuer_namespace,
        }
    }
}
