use crate::{
    domain::{
        dtos::{
            account_type::AccountType,
            profile::Profile,
            related_accounts::RelatedAccounts,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            webhook::{PayloadId, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{
            AccountDeletion, ResourceAuditLogRegistration, WebHookRegistration,
        },
    },
    use_cases::shared::audit::emit_resource_audit_event,
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use serde_json::json;
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_my_account",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty
    ),
    skip(
        profile,
        account_deletion_repo,
        webhook_registration_repo,
        audit_repo
    )
)]
pub async fn delete_my_account(
    profile: Profile,
    account_deletion_repo: Box<&dyn AccountDeletion>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .soft_delete_account(
            profile.acc_id,
            AccountType::User,
            RelatedAccounts::AllowedAccounts(vec![profile.acc_id]),
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    if let DeletionResponseKind::Deleted = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Account,
            profile.acc_id,
            None,
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            json!({ "action": "delete_my_account" }),
        )
        .await;

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountDeleted,
            json!({ "id": profile.acc_id }),
            PayloadId::Uuid(profile.acc_id),
            webhook_registration_repo,
        )
        .instrument(span)
        .await?;

        tracing::trace!("Side effects dispatched");
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;
    use mycelium_base::entities::CreateResponseKind;

    struct FakeAccountDeletionRepo {
        response: DeletionResponseKind<Uuid>,
    }

    #[async_trait]
    impl AccountDeletion for FakeAccountDeletionRepo {
        async fn hard_delete_account(
            &self,
            _: Uuid,
            _: AccountType,
            _: RelatedAccounts,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
        }

        async fn soft_delete_account(
            &self,
            _: Uuid,
            _: AccountType,
            _: RelatedAccounts,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            Ok(self.response.to_owned())
        }

        async fn delete_account_meta(
            &self,
            _: Uuid,
            _: crate::domain::dtos::account::AccountMetaKey,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
        }
    }

    struct FakeWebHookRegistrationRepo;

    #[async_trait]
    impl WebHookRegistration for FakeWebHookRegistrationRepo {
        async fn create(
            &self,
            _: crate::domain::dtos::webhook::WebHook,
        ) -> Result<
            CreateResponseKind<crate::domain::dtos::webhook::WebHook>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn register_execution_event(
            &self,
            _: crate::domain::dtos::webhook::WebHookPayloadArtifact,
        ) -> Result<CreateResponseKind<Uuid>, MappedErrors> {
            Ok(CreateResponseKind::Created(Uuid::new_v4()))
        }
    }

    #[tokio::test]
    async fn delete_my_account_emits_audit_event_on_success() {
        let profile = Profile::default();
        let account_deletion_repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::Deleted,
        };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let acc_id = profile.acc_id;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == acc_id
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Deleted
            })
            .returning(|_| Ok(()));

        let result = delete_my_account(
            profile,
            Box::new(&account_deletion_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_my_account_does_not_emit_audit_event_when_not_deleted() {
        let profile = Profile::default();
        let account_deletion_repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::NotDeleted(
                profile.acc_id,
                "not deleted".to_string(),
            ),
        };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_my_account(
            profile,
            Box::new(&account_deletion_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
