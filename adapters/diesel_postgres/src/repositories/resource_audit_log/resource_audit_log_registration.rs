use myc_core::domain::{
    dtos::resource_audit_log::NewResourceAuditLogEvent,
    entities::ResourceAuditLogRegistration,
};
use mycelium_base::utils::errors::MappedErrors;

use async_trait::async_trait;
use shaku::Component;
use tokio::sync::mpsc;

/// Non-blocking enqueue into the audit log channel. Does no I/O itself --
/// the single background dispatcher (wired in `ports/api`) drains this
/// channel and performs the actual insert.
///
/// The `sender` field is deliberately NOT `#[shaku(inject)]`-resolved: it is
/// supplied via `.with_component_parameters::<ResourceAuditLogRegistrationSqlDbRepository>(...)`
/// at `SqlAppModule::builder()` time, exactly like `DieselDbPoolProvider`'s
/// `pool` field.
#[derive(Component)]
#[shaku(interface = ResourceAuditLogRegistration)]
pub struct ResourceAuditLogRegistrationSqlDbRepository {
    sender: mpsc::Sender<NewResourceAuditLogEvent>,
}

#[async_trait]
impl ResourceAuditLogRegistration
    for ResourceAuditLogRegistrationSqlDbRepository
{
    #[tracing::instrument(name = "create_resource_audit_log_event", skip_all)]
    async fn create(
        &self,
        event: NewResourceAuditLogEvent,
    ) -> Result<(), MappedErrors> {
        match self.sender.try_send(event) {
            Ok(_) => {}
            Err(mpsc::error::TrySendError::Full(e)) => {
                tracing::warn!(
                    resource_type = ?e.resource_type,
                    resource_id = %e.resource_id,
                    "resource audit log channel full, dropping event"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!(
                    "resource audit log channel closed, dropping event"
                );
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use myc_core::domain::dtos::resource_audit_log::{
        ResourceAuditEventKind, ResourceAuditResourceType,
    };
    use myc_core::domain::dtos::written_by::WrittenBy;
    use uuid::Uuid;

    fn sample_event() -> NewResourceAuditLogEvent {
        NewResourceAuditLogEvent {
            resource_type: ResourceAuditResourceType::Account,
            resource_id: Uuid::new_v4(),
            tenant_id: None,
            event: ResourceAuditEventKind::Created,
            performed_by: WrittenBy::new_anemic(),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_never_blocks_and_always_returns_ok() {
        let (sender, mut receiver) =
            mpsc::channel::<NewResourceAuditLogEvent>(1);
        let repo = ResourceAuditLogRegistrationSqlDbRepository { sender };

        let first_event = sample_event();
        let first_event_id = first_event.resource_id;

        let first_result = repo.create(first_event).await;
        assert!(first_result.is_ok());

        // Capacity is 1 and nothing has drained the channel yet, so this
        // second call must hit `Full` -- and still return `Ok(())`.
        let second_result = repo.create(sample_event()).await;
        assert!(second_result.is_ok());

        let received = receiver.try_recv().expect("expected the first event");
        assert_eq!(received.resource_id, first_event_id);

        // Only the first event made it onto the channel.
        assert!(receiver.try_recv().is_err());
    }
}
