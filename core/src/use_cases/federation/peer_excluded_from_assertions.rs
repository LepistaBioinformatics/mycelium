use crate::domain::dtos::federation::{
    FederationPeer, PeerStatus, PROFILE_ASSERTION_KIND,
};

/// Why the peer must never contribute, if it must not: an operator or
/// handshake decision, or nothing negotiated or allowed to assert
pub(super) fn excluded(peer: &FederationPeer) -> Option<String> {
    if matches!(peer.status, PeerStatus::Quarantined | PeerStatus::Untrusted) {
        return Some(format!("peer is {}", peer.status.as_str()));
    }
    if !peer
        .negotiated_kinds
        .iter()
        .any(|kind| kind == PROFILE_ASSERTION_KIND)
    {
        return Some(format!("{PROFILE_ASSERTION_KIND} not negotiated"));
    }
    if !peer.policy.may_assert {
        return Some("peer policy does not allow assertions".to_string());
    }

    None
}
