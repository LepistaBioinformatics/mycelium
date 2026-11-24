use super::SubjectRef;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// Why an issuer pushes a tombstone (FED-22)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TombstoneReason {
    /// The subject lost access on the issuer; the link stays
    GrantsChanged,

    /// The link was revoked on the issuer; the receiver revokes its copy
    LinkRevoked,
}

impl TombstoneReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GrantsChanged => "grantsChanged",
            Self::LinkRevoked => "linkRevoked",
        }
    }
}

impl fmt::Display for TombstoneReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TombstoneReason {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "grantsChanged" => Ok(Self::GrantsChanged),
            "linkRevoked" => Ok(Self::LinkRevoked),
            other => Err(format!("unknown tombstone reason: {other}")),
        }
    }
}

/// "What I asserted for this subject before `issued_at` no longer holds"
///
/// Pushed by the issuer inside a signed peer request (FED-22, FED-23).
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tombstone {
    /// The receiver's `local_subject_ref` of the link
    pub subject_ref: SubjectRef,
    pub reason: TombstoneReason,
    pub issued_at: DateTime<Local>,
}

impl Tombstone {
    /// Whether an assertion issued at `issued_at` predates this tombstone
    pub fn supersedes(&self, issued_at: DateTime<Local>) -> bool {
        issued_at <= self.issued_at
    }
}

/// A tombstone waiting to be pushed to a peer
#[derive(Clone, Debug, PartialEq)]
pub struct PendingRevocation {
    pub peer_id: Uuid,
    pub tombstone: Tombstone,
}

/// A row of the issuer's outbox (FED-24)
#[derive(Clone, Debug, PartialEq)]
pub struct OutboxEntry {
    pub id: Uuid,
    pub revocation: PendingRevocation,
    pub attempts: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;

    #[test]
    fn the_reason_round_trips_and_an_older_assertion_is_superseded() {
        for reason in
            [TombstoneReason::GrantsChanged, TombstoneReason::LinkRevoked]
        {
            assert_eq!(reason.as_str().parse::<TombstoneReason>(), Ok(reason));
        }

        let tombstone = Tombstone {
            subject_ref: SubjectRef::mint(),
            reason: TombstoneReason::GrantsChanged,
            issued_at: Local::now(),
        };

        assert!(tombstone.supersedes(tombstone.issued_at));
        assert!(
            !tombstone.supersedes(tombstone.issued_at + Duration::seconds(1))
        );
    }
}
