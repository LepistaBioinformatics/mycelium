// ? ---------------------------------------------------------------------------
// ? NewResourceAuditLogEvent
//
// The pre-insert shape a use case builds and hands to the
// `ResourceAuditLogRegistration` port -- identical to `ResourceAuditLog`
// minus `id`, since the row's own identifier does not exist until the
// dispatcher performs the actual insert.
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
pub struct NewResourceAuditLogEvent {
    /// The coarse category of the resource this event describes.
    pub resource_type: ResourceAuditResourceType,

    /// The identifier of the affected resource.
    pub resource_id: Uuid,

    /// The tenant the affected resource belongs to, when applicable.
    pub tenant_id: Option<Uuid>,

    /// The kind of event this row describes.
    pub event: ResourceAuditEventKind,

    /// Who performed the action.
    pub performed_by: WrittenBy,

    /// Arbitrary, use-case-defined context about the event.
    pub metadata: serde_json::Value,

    /// The moment the triggering operation succeeded -- captured
    /// synchronously, not derived from insert time.
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_resource_audit_log_event_round_trips_through_json() {
        let event = NewResourceAuditLogEvent {
            resource_type: ResourceAuditResourceType::Webhook,
            resource_id: Uuid::new_v4(),
            tenant_id: None,
            event: ResourceAuditEventKind::Updated,
            performed_by: WrittenBy::new_from_account(Uuid::new_v4()),
            metadata: serde_json::json!({ "field": "is_active" }),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: NewResourceAuditLogEvent =
            serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.resource_type, event.resource_type);
        assert_eq!(parsed.resource_id, event.resource_id);
        assert_eq!(parsed.tenant_id, event.tenant_id);
        assert_eq!(parsed.event, event.event);
        assert_eq!(parsed.performed_by, event.performed_by);
        assert_eq!(parsed.metadata, event.metadata);
        assert_eq!(parsed.created_at, event.created_at);
    }
}
