use crate::{
    domain::{
        dtos::{
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{ResourceAuditLogRegistration, UserDeletion},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::error;
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_default_user",
    skip(user_deletion_repo, audit_repo)
)]
pub(super) async fn delete_default_user(
    user_id: Uuid,
    user_deletion_repo: Box<&dyn UserDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<(), MappedErrors> {
    match user_deletion_repo.delete(user_id).await? {
        DeletionResponseKind::Deleted => {
            emit_resource_audit_event(
                audit_repo,
                ResourceAuditResourceType::User,
                user_id,
                None,
                ResourceAuditEventKind::Deleted,
                WrittenBy::new_from_user(user_id),
                serde_json::json!({ "action": "delete_default_user" }),
            )
            .await;

            Ok(())
        }
        DeletionResponseKind::NotDeleted(id, msg) => {
            error!("Unable to delete user: {}. Error: {}", id.to_string(), msg);

            use_case_err(format!(
                "Unable to delete user: {}. Error: {}",
                id, msg
            ))
            .as_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;

    struct MockUserDeletionRepo {
        should_fail: bool,
    }

    #[async_trait]
    impl UserDeletion for MockUserDeletionRepo {
        async fn delete(
            &self,
            user_id: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            if self.should_fail {
                return Ok(DeletionResponseKind::NotDeleted(
                    user_id,
                    "simulated deletion failure".to_string(),
                ));
            }

            Ok(DeletionResponseKind::Deleted)
        }
    }

    #[tokio::test]
    async fn delete_default_user_emits_audit_event_on_success() {
        let user_id = Uuid::new_v4();
        let deletion_repo = MockUserDeletionRepo { should_fail: false };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::User
                    && event.resource_id == user_id
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Deleted
            })
            .returning(|_| Ok(()));

        let result = delete_default_user(
            user_id,
            Box::new(&deletion_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_default_user_does_not_emit_audit_event_on_failure() {
        let user_id = Uuid::new_v4();
        let deletion_repo = MockUserDeletionRepo { should_fail: true };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_default_user(
            user_id,
            Box::new(&deletion_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
