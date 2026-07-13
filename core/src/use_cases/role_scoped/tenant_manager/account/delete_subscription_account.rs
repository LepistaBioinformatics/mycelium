use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account_type::AccountType,
            guest_role::Permission,
            profile::Profile,
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
    name = "delete_subscription_account",
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
pub async fn delete_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
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

    let related_accounts = profile
        .on_tenant(tenant_id.to_owned())
        .on_account(account_id)
        .with_system_accounts_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_related_accounts_or_tenant_wide_permission_or_error(
            tenant_id,
            Permission::Write,
        )?;

    // ? -----------------------------------------------------------------------
    // ? Delete account
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .soft_delete_account(
            account_id,
            AccountType::Subscription { tenant_id },
            related_accounts,
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
            account_id,
            Some(tenant_id),
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            json!({ "action": "delete_subscription_account" }),
        )
        .await;

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::SubscriptionAccountDeleted,
            json!({ "id": account_id }),
            PayloadId::Uuid(account_id),
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
    use crate::domain::{
        dtos::related_accounts::RelatedAccounts,
        entities::MockResourceAuditLogRegistration,
    };

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

    fn staff_profile() -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            true,
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
    async fn delete_subscription_account_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = staff_profile();
        let account_deletion_repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::Deleted,
        };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Deleted
            })
            .returning(|_| Ok(()));

        let result = delete_subscription_account(
            profile,
            tenant_id,
            account_id,
            Box::new(&account_deletion_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_subscription_account_does_not_emit_audit_event_on_permission_error(
    ) {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = Profile::default();
        let account_deletion_repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::Deleted,
        };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_subscription_account(
            profile,
            tenant_id,
            account_id,
            Box::new(&account_deletion_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_subscription_account_does_not_emit_audit_event_when_not_deleted(
    ) {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = staff_profile();
        let account_deletion_repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::NotDeleted(
                account_id,
                "not deleted".to_string(),
            ),
        };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_subscription_account(
            profile,
            tenant_id,
            account_id,
            Box::new(&account_deletion_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
