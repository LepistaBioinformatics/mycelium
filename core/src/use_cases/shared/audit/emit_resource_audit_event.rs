// ? ---------------------------------------------------------------------------
// ? emit_resource_audit_event
//
// The one greppable call site every write use case calls to record a
// resource-audit-trail event. Builds `NewResourceAuditLogEvent`, captures
// `created_at` synchronously (at the moment the caller's real mutation
// already succeeded, keeping ordering/timestamps correct despite the async
// write path downstream), and swallows any port error into a traced warning
// instead of propagating it -- audit logging must never fail the caller's
// own operation.
// ? ---------------------------------------------------------------------------

use crate::domain::{
    dtos::{
        resource_audit_log::{
            NewResourceAuditLogEvent, ResourceAuditEventKind,
            ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::ResourceAuditLogRegistration,
};

use chrono::Utc;
use uuid::Uuid;

#[tracing::instrument(
    name = "emit_resource_audit_event",
    skip_all,
    fields(?resource_type, %resource_id, ?event)
)]
pub async fn emit_resource_audit_event(
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
    resource_type: ResourceAuditResourceType,
    resource_id: Uuid,
    tenant_id: Option<Uuid>,
    event: ResourceAuditEventKind,
    performed_by: WrittenBy,
    metadata: serde_json::Value,
) {
    let new_event = NewResourceAuditLogEvent {
        resource_type: resource_type.to_owned(),
        resource_id,
        tenant_id,
        event,
        performed_by,
        metadata,
        created_at: Utc::now(),
    };

    if let Err(e) = audit_repo.create(new_event).await {
        tracing::warn!(
            error = ?e,
            ?resource_type,
            %resource_id,
            "failed to enqueue resource audit event"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use mycelium_base::utils::errors::use_case_err;

    #[tokio::test]
    async fn emit_resource_audit_event_calls_create_once_with_expected_fields()
    {
        let resource_id = Uuid::new_v4();
        let tenant_id = Some(Uuid::new_v4());
        let performed_by = WrittenBy::new_from_account(Uuid::new_v4());
        let metadata =
            serde_json::json!({ "action": "create_subscription_account" });

        let expected_performed_by = performed_by.to_owned();
        let expected_metadata = metadata.to_owned();

        let mut mock = MockResourceAuditLogRegistration::new();
        mock.expect_create()
            .times(1)
            .withf(move |event: &NewResourceAuditLogEvent| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == resource_id
                    && event.tenant_id == tenant_id
                    && event.event == ResourceAuditEventKind::Created
                    && event.performed_by == expected_performed_by
                    && event.metadata == expected_metadata
            })
            .returning(|_| Ok(()));

        emit_resource_audit_event(
            Box::new(&mock),
            ResourceAuditResourceType::Account,
            resource_id,
            tenant_id,
            ResourceAuditEventKind::Created,
            performed_by,
            metadata,
        )
        .await;
    }

    #[tokio::test]
    async fn emit_resource_audit_event_swallows_port_error() {
        let resource_id = Uuid::new_v4();

        let mut mock = MockResourceAuditLogRegistration::new();
        mock.expect_create().times(1).returning(|_| {
            use_case_err("simulated audit repository failure").as_error()
        });

        let result = emit_resource_audit_event(
            Box::new(&mock),
            ResourceAuditResourceType::Webhook,
            resource_id,
            None,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_anemic(),
            serde_json::json!({}),
        )
        .await;

        assert_eq!(result, ());
    }
}
