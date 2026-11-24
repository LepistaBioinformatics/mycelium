use super::SubjectRef;
use crate::domain::dtos::written_by::WrittenBy;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// Where an identity link stands (D-2)
///
/// `Pending` exists only on the issuer, between recording the link and the
/// consumer's acknowledgement. Stored as its lowercase name.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkStatus {
    Pending,
    Active,
    Revoked,
}

impl LinkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }
}

impl fmt::Display for LinkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LinkStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            other => Err(format!("unknown link status: {other}")),
        }
    }
}

/// A local user linked to the same person's account on a peer (FED-12)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityLink {
    pub id: Uuid,
    pub local_user_id: Uuid,
    pub local_email: String,
    pub peer_id: Uuid,

    /// How this instance names the user when asking the peer; `None` while
    /// pending
    pub peer_subject_ref: Option<SubjectRef>,

    /// How the peer names the user when asking this instance
    pub local_subject_ref: SubjectRef,

    pub status: LinkStatus,
    pub created: DateTime<Local>,
    pub established_at: Option<DateTime<Local>>,
    pub revoked_at: Option<DateTime<Local>>,
    pub revoked_by: Option<WrittenBy>,
}

/// A link to record
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewIdentityLink {
    pub local_user_id: Uuid,
    pub local_email: String,
    pub peer_id: Uuid,
    pub peer_subject_ref: Option<SubjectRef>,
    pub status: LinkStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips_through_its_stored_name_and_json() {
        for status in
            [LinkStatus::Pending, LinkStatus::Active, LinkStatus::Revoked]
        {
            assert_eq!(status.as_str().parse::<LinkStatus>(), Ok(status));
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                serde_json::json!(status.as_str())
            );
        }

        assert!("linked".parse::<LinkStatus>().is_err());
    }
}
