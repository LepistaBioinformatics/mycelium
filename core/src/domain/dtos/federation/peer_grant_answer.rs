use super::VerifiedAssertion;

/// What asking one peer for a subject's grants produced
///
/// A failed call (connection, timeout, non-2xx) is not an answer: it is the
/// error of the port, and counts against the peer's circuit.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PeerGrantAnswer {
    Verified(VerifiedAssertion),
    Unknown,
    /// An assertion came back but failed verification
    Invalid(String),
}
