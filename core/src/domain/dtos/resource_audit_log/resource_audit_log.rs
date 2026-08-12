// ? ---------------------------------------------------------------------------
// ? ResourceAuditLog
//
// One persisted, immutable row of the resource audit trail. Immutability is
// enforced both here (no `Updating`/`Deletion` port exists for this
// resource) and at the database level (a trigger rejects UPDATE/DELETE
// regardless of who issues the SQL). See `NewResourceAuditLogEvent` for the
// pre-insert shape a use case builds before this row's `id` exists.
// ? ---------------------------------------------------------------------------

use super::resource_audit_event_kind::ResourceAuditEventKind;
use super::resource_audit_resource_type::ResourceAuditResourceType;
use crate::domain::dtos::written_by::WrittenBy;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAuditLog {
    /// The audit row's own identifier.
    pub id: Uuid,

    /// The coarse category of the resource this row describes.
    pub resource_type: ResourceAuditResourceType,

    /// The identifier of the affected resource.
    pub resource_id: Uuid,

    /// The tenant the affected resource belongs to, when applicable.
    pub tenant_id: Option<Uuid>,

    /// The kind of event this row describes.
    pub event: ResourceAuditEventKind,

    /// Who performed the action.
    pub performed_by: WrittenBy,

    /// Arbitrary, use-case-defined context about the event (e.g. what
    /// changed on an `Updated` event).
    pub metadata: serde_json::Value,

    /// The moment the triggering operation succeeded -- captured
    /// synchronously by the caller, not derived from insert time.
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_audit_log_round_trips_through_json() {
        let log = ResourceAuditLog {
            id: Uuid::new_v4(),
            resource_type: ResourceAuditResourceType::Account,
            resource_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            event: ResourceAuditEventKind::Created,
            performed_by: WrittenBy::new_from_user(Uuid::new_v4()),
            metadata: serde_json::json!({ "action": "create_subscription_account" }),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&log).unwrap();
        let parsed: ResourceAuditLog = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, log.id);
        assert_eq!(parsed.resource_type, log.resource_type);
        assert_eq!(parsed.resource_id, log.resource_id);
        assert_eq!(parsed.tenant_id, log.tenant_id);
        assert_eq!(parsed.event, log.event);
        assert_eq!(parsed.performed_by, log.performed_by);
        assert_eq!(parsed.metadata, log.metadata);
        assert_eq!(parsed.created_at, log.created_at);
    }

    #[test]
    fn resource_audit_log_serializes_fields_as_camel_case() {
        let log = ResourceAuditLog {
            id: Uuid::from_u128(0),
            resource_type: ResourceAuditResourceType::Tenant,
            resource_id: Uuid::from_u128(0),
            tenant_id: None,
            event: ResourceAuditEventKind::Deleted,
            performed_by: WrittenBy::new_anemic(),
            metadata: serde_json::json!({}),
            created_at: DateTime::<Utc>::UNIX_EPOCH,
        };

        let json = serde_json::to_value(&log).unwrap();
        let object = json.as_object().unwrap();

        assert!(object.contains_key("resourceType"));
        assert!(object.contains_key("resourceId"));
        assert!(object.contains_key("tenantId"));
        assert!(object.contains_key("performedBy"));
        assert!(object.contains_key("createdAt"));
    }
}
