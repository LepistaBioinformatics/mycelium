use myc_core::domain::{
    dtos::resource_audit_log::NewResourceAuditLogEvent,
    entities::ResourceAuditLogRegistration,
};
use mycelium_base::utils::errors::MappedErrors;

use async_trait::async_trait;
use shaku::Component;
use tokio::sync::mpsc::{error::TrySendError, Sender};

/// Non-blocking enqueue into the audit log channel. Does no I/O itself --
/// the actual insert happens in `resource_audit_log_dispatcher` (ports/api),
/// the channel's single consumer. `sender` is a plain (non-`#[shaku(inject)]`)
/// field, following `DieselSqliteDbPoolProvider`'s externally-parameterized
/// `Component` pattern: shaku generates a
/// `ResourceAuditLogRegistrationSqlDbRepositoryParameters` struct so the
/// caller supplies `sender` via `.with_component_parameters(...)` at
/// `SqlAppModule::builder()` time instead of DI-resolving it.
#[derive(Component)]
#[shaku(interface = ResourceAuditLogRegistration)]
pub struct ResourceAuditLogRegistrationSqlDbRepository {
    pub sender: Sender<NewResourceAuditLogEvent>,
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
            Err(TrySendError::Full(dropped)) => {
                tracing::warn!(
                    resource_type = ?dropped.resource_type,
                    resource_id = %dropped.resource_id,
                    "audit log channel full, dropping event"
                );
            }
            Err(TrySendError::Closed(_)) => {
                tracing::error!("audit log channel closed, dropping event");
            }
        }

        Ok(())
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::{
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    };
    use uuid::Uuid;

    fn sample_event() -> NewResourceAuditLogEvent {
        NewResourceAuditLogEvent {
            resource_type: ResourceAuditResourceType::Account,
            resource_id: Uuid::new_v4(),
            tenant_id: None,
            event: ResourceAuditEventKind::Created,
            performed_by: WrittenBy::new_anemic(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_never_blocks_and_always_returns_ok(
    ) -> Result<(), MappedErrors> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let repository = ResourceAuditLogRegistrationSqlDbRepository { sender };

        // First event: channel has room, `try_send` succeeds.
        let first = sample_event();
        let first_resource_id = first.resource_id;
        repository.create(first).await?;

        // Second event: channel is now full (capacity 1, nothing drained
        // yet), so `try_send` hits `Full` -- must still return `Ok(())`
        // and must not panic.
        repository.create(sample_event()).await?;

        let received = receiver
            .try_recv()
            .expect("expected the first event to have been enqueued");
        assert_eq!(received.resource_id, first_resource_id);

        // Only the first event made it onto the channel.
        assert!(receiver.try_recv().is_err());

        Ok(())
    }
}
