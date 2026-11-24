use crate::domain::dtos::federation::PeerStatus;

use std::time::Duration;

/// How the last call to a peer went
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactOutcome {
    Success,
    Failure,
}

/// The status a peer moves to after a call (FED-21)
///
/// `failure_count` is the count *after* recording this outcome. Success
/// closes the circuit; failures degrade the peer until the count reaches
/// `max_failures_before_open`, which opens it. Quarantined and untrusted
/// peers are operator or handshake decisions the circuit never overrides.
///
pub fn next_peer_status(
    current: PeerStatus,
    outcome: ContactOutcome,
    failure_count: u32,
    max_failures_before_open: u32,
) -> PeerStatus {
    match (current, outcome) {
        (PeerStatus::Quarantined | PeerStatus::Untrusted, _) => current,
        (_, ContactOutcome::Success) => PeerStatus::Active,
        _ if failure_count >= max_failures_before_open => {
            PeerStatus::Unreachable
        }
        _ => PeerStatus::Degraded,
    }
}

/// Whether the peer may be called now
///
/// An open circuit (`unreachable`) lets one probe through once `cool_down`
/// has passed since it opened (half-open). Quarantined and untrusted peers
/// are never called.
///
pub fn peer_may_be_called(
    status: PeerStatus,
    since_status_change: Duration,
    cool_down: Duration,
) -> bool {
    match status {
        PeerStatus::Quarantined | PeerStatus::Untrusted => false,
        PeerStatus::Unreachable => since_status_change >= cool_down,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ContactOutcome::{Failure, Success};
    use PeerStatus::*;

    const THRESHOLD: u32 = 3;

    #[test]
    fn next_status_table() {
        let table = [
            // (current, outcome, failures after, expected)
            (Declared, Success, 0, Active),
            (Degraded, Success, 0, Active),
            (Unreachable, Success, 0, Active),
            (Active, Failure, 1, Degraded),
            (Degraded, Failure, 2, Degraded),
            (Degraded, Failure, 3, Unreachable),
            (Unreachable, Failure, 7, Unreachable),
            (Quarantined, Success, 0, Quarantined),
            (Quarantined, Failure, 9, Quarantined),
            (Untrusted, Success, 0, Untrusted),
        ];

        for (current, outcome, failures, expected) in table {
            assert_eq!(
                next_peer_status(current, outcome, failures, THRESHOLD),
                expected,
                "{current} after {outcome:?} with {failures} failures"
            );
        }
    }

    #[test]
    fn may_be_called_table() {
        let cool_down = Duration::from_secs(60);
        let table = [
            // (status, seconds since the change, expected)
            (Active, 0, true),
            (Degraded, 0, true),
            (Declared, 0, true),
            (Unreachable, 59, false),
            (Unreachable, 60, true),
            (Quarantined, 3600, false),
            (Untrusted, 3600, false),
        ];

        for (status, seconds, expected) in table {
            assert_eq!(
                peer_may_be_called(
                    status,
                    Duration::from_secs(seconds),
                    cool_down
                ),
                expected,
                "{status} after {seconds}s"
            );
        }
    }
}
