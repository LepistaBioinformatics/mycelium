use super::{FederatedResolution, PeerOutcome};

use serde::{Deserialize, Serialize};

/// A subject's federated grants as answered to them (FED-13, FED-17, FED-19)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFederatedGrants {
    /// Why the subject gets no federated grant at all: a local denial wins
    pub denied: Option<String>,
    /// Some linked peer contributed nothing because of a failure
    pub partial: bool,
    /// Some grants come from an assertion its peer could not refresh
    pub stale: bool,
    pub peers: Vec<PeerOutcome>,
}

impl ResolvedFederatedGrants {
    pub fn denied(reason: String) -> Self {
        Self {
            denied: Some(reason),
            partial: false,
            stale: false,
            peers: vec![],
        }
    }
}

impl From<FederatedResolution> for ResolvedFederatedGrants {
    fn from(resolution: FederatedResolution) -> Self {
        Self {
            denied: None,
            partial: resolution.is_partial(),
            stale: resolution.is_stale(),
            peers: resolution.0,
        }
    }
}
