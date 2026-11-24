use super::{FederationPeer, PeerStatus};

/// Whether a caller claiming to be a peer may be served at all
///
/// Decided before any key lookup, so an undeclared caller costs one indexed
/// read and nothing more.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PeerAdmission {
    /// No peer is declared with this `instance_id`
    Unknown,

    /// Declared, but its status forbids serving it
    Refused(PeerStatus),

    Admitted(Box<FederationPeer>),
}
