use crate::domain::dtos::{guest_role::Permission, written_by::WrittenBy};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// Where a declared peer stands in the trust lifecycle
///
/// Stored as its lowercase name. `Untrusted` is set when a handshake sees an
/// `instanceId` other than the declared one; the peer is never silently
/// re-pinned (FED-08).
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerStatus {
    Declared,
    Handshaking,
    Active,
    Degraded,
    Unreachable,
    Quarantined,
    Untrusted,
}

impl PeerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Handshaking => "handshaking",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Unreachable => "unreachable",
            Self::Quarantined => "quarantined",
            Self::Untrusted => "untrusted",
        }
    }
}

impl fmt::Display for PeerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PeerStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "declared" => Ok(Self::Declared),
            "handshaking" => Ok(Self::Handshaking),
            "active" => Ok(Self::Active),
            "degraded" => Ok(Self::Degraded),
            "unreachable" => Ok(Self::Unreachable),
            "quarantined" => Ok(Self::Quarantined),
            "untrusted" => Ok(Self::Untrusted),
            other => Err(format!("unknown peer status: {other}")),
        }
    }
}

/// What a peer is allowed to contribute (FED-10)
///
/// The default denies everything: a peer asserts nothing until the operator
/// says otherwise. Trust is never unbounded.
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FederationPolicy {
    /// Whether the peer may assert grants at all
    pub may_assert: bool,

    /// Whether a link may join accounts whose emails differ (D-7)
    pub allow_different_emails: bool,

    /// Local roles the peer's assertions may map onto
    pub allowed_local_roles: Vec<String>,

    /// Local permissions the peer's assertions may map onto
    pub allowed_local_permissions: Vec<Permission>,
}

/// A peer declared by the operator (FED-07)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FederationPeer {
    pub id: Uuid,

    /// The identity the operator expects the peer to announce
    pub instance_id: Uuid,

    pub base_url: String,
    pub display_name: Option<String>,
    pub status: PeerStatus,

    /// Assertion kinds this instance accepts from the peer
    pub consume_kinds: Vec<String>,

    /// Assertion kinds this instance serves to the peer
    pub serve_kinds: Vec<String>,

    /// The kinds both sides support, set by the handshake
    pub negotiated_kinds: Vec<String>,

    pub policy: FederationPolicy,
    pub last_contact_at: Option<DateTime<Local>>,
    pub failure_count: u32,
    pub created: DateTime<Local>,
    pub created_by: Option<WrittenBy>,
    pub updated: Option<DateTime<Local>>,
    pub updated_by: Option<WrittenBy>,
}

/// What the operator gives to declare a peer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewFederationPeer {
    pub instance_id: Uuid,
    pub base_url: String,
    pub display_name: Option<String>,
    pub consume_kinds: Vec<String>,
    pub serve_kinds: Vec<String>,
    pub policy: FederationPolicy,
}

/// One public key of a peer, as read from its self-description
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerKey {
    pub peer_id: Uuid,
    pub kid: String,
    pub alg: String,

    /// The raw Ed25519 public key, Base64URL without padding (JWK `x`)
    pub public_key: String,

    pub fetched_at: DateTime<Local>,
    pub expires_at: Option<DateTime<Local>>,
}

/// A peer public key to store, as read from its self-description
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPeerKey {
    pub kid: String,
    pub alg: String,
    pub public_key: String,
    pub expires_at: Option<DateTime<Local>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUSES: [PeerStatus; 7] = [
        PeerStatus::Declared,
        PeerStatus::Handshaking,
        PeerStatus::Active,
        PeerStatus::Degraded,
        PeerStatus::Unreachable,
        PeerStatus::Quarantined,
        PeerStatus::Untrusted,
    ];

    #[test]
    fn status_round_trips_through_its_stored_name_and_json() {
        for status in STATUSES {
            assert_eq!(status.as_str().parse::<PeerStatus>(), Ok(status));
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                serde_json::json!(status.as_str())
            );
        }

        assert!("trusted".parse::<PeerStatus>().is_err());
    }

    #[test]
    fn the_default_policy_denies_everything() {
        let policy = FederationPolicy::default();

        assert!(!policy.may_assert);
        assert!(!policy.allow_different_emails);
        assert!(policy.allowed_local_roles.is_empty());
        assert!(policy.allowed_local_permissions.is_empty());
    }

    #[test]
    fn an_empty_stored_policy_reads_as_the_deny_all_default() {
        let policy: FederationPolicy =
            serde_json::from_value(serde_json::json!({})).unwrap();

        assert_eq!(policy, FederationPolicy::default());
    }

    #[test]
    fn policy_round_trips_in_camel_case() {
        let policy = FederationPolicy {
            may_assert: true,
            allow_different_emails: false,
            allowed_local_roles: vec!["reviewer".to_string()],
            allowed_local_permissions: vec![Permission::Read],
        };
        let json = serde_json::to_value(&policy).unwrap();

        assert_eq!(json["mayAssert"], true);
        assert_eq!(json["allowedLocalRoles"][0], "reviewer");
        assert_eq!(
            serde_json::from_value::<FederationPolicy>(json).unwrap(),
            policy
        );
    }

    #[test]
    fn peer_and_key_round_trip_through_json() {
        let peer = FederationPeer {
            id: Uuid::new_v4(),
            instance_id: Uuid::new_v4(),
            base_url: "https://myc-02.example.com".to_string(),
            display_name: Some("Instance 02".to_string()),
            status: PeerStatus::Active,
            consume_kinds: vec!["profile.v1".to_string()],
            serve_kinds: vec![],
            negotiated_kinds: vec![],
            policy: FederationPolicy::default(),
            last_contact_at: None,
            failure_count: 0,
            created: Local::now(),
            created_by: None,
            updated: None,
            updated_by: None,
        };
        let key = PeerKey {
            peer_id: peer.id,
            kid: "1".to_string(),
            alg: "EdDSA".to_string(),
            public_key: "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"
                .to_string(),
            fetched_at: Local::now(),
            expires_at: None,
        };

        let peer_json = serde_json::to_value(&peer).unwrap();
        assert_eq!(peer_json["status"], "active");
        assert_eq!(
            serde_json::from_value::<FederationPeer>(peer_json).unwrap(),
            peer
        );
        assert_eq!(
            serde_json::from_value::<PeerKey>(
                serde_json::to_value(&key).unwrap()
            )
            .unwrap(),
            key
        );
    }
}
