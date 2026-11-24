use super::FederatedGrants;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How one linked peer contributed to a resolution (FED-13, FED-16)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome", content = "reason")]
pub enum PeerResolution {
    /// Served from the assertion store, no call made
    Stored,
    /// The peer answered with a verified assertion
    Answered,
    /// The peer has no active link for the reference
    Unknown,
    /// The peer was skipped (inactive, circuit open, kind not negotiated)
    Skipped(String),
    /// The peer could not refresh its assertion; its stored grants, still
    /// within their lifetime, are served (FED-17)
    Stale(String),
    /// The call failed or did not finish before the deadline
    Unreachable(String),
    /// The assertion failed verification or the namespace guard
    Rejected(String),
}

impl PeerResolution {
    /// Whether the peer's grants are missing from the result for a reason
    /// other than the peer having none
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Unreachable(_) | Self::Rejected(_))
    }

    /// Whether the peer's grants come from an assertion it could not refresh
    pub fn is_stale(&self) -> bool {
        matches!(self, Self::Stale(_))
    }
}

/// One linked peer's share of a resolution
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerOutcome {
    pub peer_id: Uuid,
    pub instance_id: Uuid,
    pub resolution: PeerResolution,
    pub grants: FederatedGrants,
}

/// The federated part of a subject's grants, per linked peer
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FederatedResolution(pub Vec<PeerOutcome>);

impl FederatedResolution {
    /// A partial result is not an error: some peer could not contribute
    pub fn is_partial(&self) -> bool {
        self.0.iter().any(|outcome| outcome.resolution.is_partial())
    }

    /// Some grants are served from an assertion the peer could not refresh
    pub fn is_stale(&self) -> bool {
        self.0.iter().any(|outcome| outcome.resolution.is_stale())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(resolution: PeerResolution) -> PeerOutcome {
        PeerOutcome {
            peer_id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            resolution,
            grants: FederatedGrants::default(),
        }
    }

    #[test]
    fn a_timed_out_peer_makes_the_result_partial() {
        let complete = FederatedResolution(vec![
            outcome(PeerResolution::Stored),
            outcome(PeerResolution::Unknown),
        ]);
        let partial = FederatedResolution(vec![
            outcome(PeerResolution::Answered),
            outcome(PeerResolution::Unreachable("deadline".to_string())),
        ]);

        assert!(!complete.is_partial());
        assert!(partial.is_partial());
    }

    #[test]
    fn a_stale_peer_makes_the_result_stale_but_not_partial() {
        let stale = FederatedResolution(vec![
            outcome(PeerResolution::Stored),
            outcome(PeerResolution::Stale("deadline".to_string())),
        ]);

        assert!(stale.is_stale());
        assert!(!stale.is_partial());
    }

    #[test]
    fn outcomes_serialize_with_their_reason() {
        let json = serde_json::to_value(PeerResolution::Rejected(
            "foreign id collides".to_string(),
        ))
        .unwrap();

        assert_eq!(json["outcome"], "rejected");
        assert_eq!(json["reason"], "foreign id collides");
    }
}
