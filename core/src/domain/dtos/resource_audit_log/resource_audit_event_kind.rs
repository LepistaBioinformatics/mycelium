// ? ---------------------------------------------------------------------------
// ? ResourceAuditEventKind
//
// Deliberately three values only. Status/privilege changes are modeled as
// `Updated` plus a `metadata` payload describing what changed, avoiding an
// ever-growing event enum.
// ? ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResourceAuditEventKind {
    /// The resource was created.
    Created,

    /// The resource was updated.
    Updated,

    /// The resource was deleted.
    Deleted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_audit_event_kind_round_trips_through_json() {
        let variants = [
            ResourceAuditEventKind::Created,
            ResourceAuditEventKind::Updated,
            ResourceAuditEventKind::Deleted,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: ResourceAuditEventKind =
                serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn resource_audit_event_kind_uses_camel_case() {
        let json =
            serde_json::to_string(&ResourceAuditEventKind::Created).unwrap();
        assert_eq!(json, r#""created""#);
    }
}
