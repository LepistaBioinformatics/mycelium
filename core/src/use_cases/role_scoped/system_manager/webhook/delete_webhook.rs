use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{ResourceAuditLogRegistration, WebHookDeletion},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_webhook",
    skip(profile, webhook_deletion_repo, audit_repo)
)]
pub async fn delete_webhook(
    profile: Profile,
    hook_id: Uuid,
    webhook_deletion_repo: Box<&dyn WebHookDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Delete webhook
    // ? -----------------------------------------------------------------------

    let response = webhook_deletion_repo.delete(hook_id).await?;

    if let DeletionResponseKind::Deleted = response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Webhook,
            hook_id,
            None,
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "delete_webhook" }),
        )
        .await;
    }

    Ok(response)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            profile::{Owner, Profile},
            resource_audit_log::NewResourceAuditLogEvent,
        },
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use mycelium_base::utils::errors::use_case_err;
    use shaku::Component;
    use std::str::FromStr;

    #[derive(Component)]
    #[shaku(interface = WebHookDeletion)]
    struct MockWebHookDeletionRepo {
        pub generate_error: bool,
    }

    #[async_trait]
    impl WebHookDeletion for MockWebHookDeletionRepo {
        async fn delete(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            match self.generate_error {
                true => {
                    return use_case_err("Error while deleting webhook.")
                        .as_error()
                }
                false => Ok(DeletionResponseKind::Deleted),
            }
        }
    }

    fn system_manager_profile() -> Profile {
        Profile::new(
            vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "username@domain.com".to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
                is_principal: true,
            }],
            Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0").unwrap(),
            false,
            true,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn delete_webhook_emits_audit_event_on_success() {
        let hook_id = Uuid::new_v4();

        let mut mock_audit_repo = MockResourceAuditLogRegistration::new();
        mock_audit_repo
            .expect_create()
            .times(1)
            .withf(move |event: &NewResourceAuditLogEvent| {
                event.resource_type == ResourceAuditResourceType::Webhook
                    && event.resource_id == hook_id
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Deleted
            })
            .returning(|_| Ok(()));

        let response = delete_webhook(
            system_manager_profile(),
            hook_id,
            Box::new(&MockWebHookDeletionRepo {
                generate_error: false,
            }),
            Box::new(&mock_audit_repo),
        )
        .await
        .unwrap();

        assert!(matches!(response, DeletionResponseKind::Deleted));
    }

    #[tokio::test]
    async fn delete_webhook_does_not_emit_audit_event_on_error() {
        let hook_id = Uuid::new_v4();

        let mut mock_audit_repo = MockResourceAuditLogRegistration::new();
        mock_audit_repo.expect_create().times(0);

        let response = delete_webhook(
            system_manager_profile(),
            hook_id,
            Box::new(&MockWebHookDeletionRepo {
                generate_error: true,
            }),
            Box::new(&mock_audit_repo),
        )
        .await;

        assert!(response.is_err());
    }
}
