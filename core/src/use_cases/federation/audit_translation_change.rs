use super::TranslationPorts;
use crate::domain::dtos::{
    resource_audit_log::{
        NewResourceAuditLogEvent, ResourceAuditEventKind,
        ResourceAuditResourceType,
    },
    written_by::WrittenBy,
};

use chrono::Utc;
use uuid::Uuid;

/// Record a change to a translation rule (FED-48); a failure is logged
pub(super) async fn audit_translation_change(
    rule_id: Uuid,
    event: ResourceAuditEventKind,
    metadata: serde_json::Value,
    author: &WrittenBy,
    ports: TranslationPorts<'_>,
) {
    let event = NewResourceAuditLogEvent {
        resource_type: ResourceAuditResourceType::FederationTranslation,
        resource_id: rule_id,
        tenant_id: None,
        event,
        performed_by: author.clone(),
        metadata,
        created_at: Utc::now(),
    };

    if let Err(err) = ports.audit.create(event).await {
        tracing::error!(%err, %rule_id, "translation change not audited");
    }
}
