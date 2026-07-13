use crate::domain::{
    dtos::{
        account_type::AccountType,
        profile::Profile,
        related_accounts::RelatedAccounts,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{AccountDeletion, ResourceAuditLogRegistration},
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tenant_manager_account",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty
    ),
    skip(profile, account_deletion_repo, audit_repo)
)]
pub async fn delete_tenant_manager_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_deletion_repo: Box<&dyn AccountDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Delete account
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .soft_delete_account(
            account_id,
            AccountType::TenantManager { tenant_id },
            RelatedAccounts::AllowedAccounts(vec![account_id]),
        )
        .await?;

    if let DeletionResponseKind::Deleted = &response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Account,
            account_id,
            Some(tenant_id),
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "delete_tenant_manager_account" }),
        )
        .await;
    }

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::profile::{TenantOwnership, TenantsOwnership},
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use chrono::Local;

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

    fn profile_owning_tenant(tenant_id: Uuid) -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            Some(TenantsOwnership::Records(vec![TenantOwnership {
                id: tenant_id,
                name: "Tenant Name".to_string(),
                since: Local::now(),
            }])),
        )
    }

    #[tokio::test]
    async fn delete_tenant_manager_account_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::Deleted,
        };

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

        let result = delete_tenant_manager_account(
            profile,
            tenant_id,
            account_id,
            Box::new(&repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_tenant_manager_account_does_not_emit_audit_event_on_permission_error(
    ) {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = Profile::default();
        let repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::Deleted,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_tenant_manager_account(
            profile,
            tenant_id,
            account_id,
            Box::new(&repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_tenant_manager_account_does_not_emit_audit_event_when_not_deleted(
    ) {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let repo = FakeAccountDeletionRepo {
            response: DeletionResponseKind::NotDeleted(
                account_id,
                "not deleted".to_string(),
            ),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_tenant_manager_account(
            profile,
            tenant_id,
            account_id,
            Box::new(&repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
