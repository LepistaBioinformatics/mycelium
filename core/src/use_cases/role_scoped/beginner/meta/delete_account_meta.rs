use crate::{
    domain::{
        dtos::{
            account::AccountMetaKey,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{AccountDeletion, ResourceAuditLogRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_account_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, account_deletion_repo, audit_repo)
)]
pub async fn delete_account_meta(
    profile: Profile,
    key: AccountMetaKey,
    account_deletion_repo: Box<&dyn AccountDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .delete_account_meta(profile.acc_id, key.to_owned())
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Emit the audit event
    // ? -----------------------------------------------------------------------

    if let DeletionResponseKind::Deleted = &response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::AccountMeta,
            profile.acc_id,
            None,
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "delete_account_meta",
                "meta_key": key.to_string(),
            }),
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{account_type::AccountType, related_accounts::RelatedAccounts},
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use mycelium_base::utils::errors::use_case_err;

    struct MockAccountDeletion {
        should_fail: bool,
        should_not_delete: bool,
    }

    #[async_trait]
    impl AccountDeletion for MockAccountDeletion {
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
            unimplemented!()
        }

        async fn delete_account_meta(
            &self,
            _: Uuid,
            _: AccountMetaKey,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            if self.should_fail {
                return use_case_err("simulated deletion failure").as_error();
            }

            if self.should_not_delete {
                return Ok(DeletionResponseKind::NotDeleted(
                    Uuid::new_v4(),
                    "not found".to_string(),
                ));
            }

            Ok(DeletionResponseKind::Deleted)
        }
    }

    #[tokio::test]
    async fn delete_account_meta_emits_audit_event_on_success() {
        let profile = Profile::default();
        let account_id = profile.acc_id;

        let deletion = MockAccountDeletion {
            should_fail: false,
            should_not_delete: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::AccountMeta
                    && event.event == ResourceAuditEventKind::Deleted
                    && event.resource_id == account_id
                    && event.tenant_id.is_none()
            })
            .returning(|_| Ok(()));

        let result = delete_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            Box::new(&deletion as &dyn AccountDeletion),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_account_meta_does_not_emit_audit_event_on_failure() {
        let profile = Profile::default();

        let deletion = MockAccountDeletion {
            should_fail: true,
            should_not_delete: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            Box::new(&deletion as &dyn AccountDeletion),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_account_meta_does_not_emit_audit_event_when_not_deleted() {
        let profile = Profile::default();

        let deletion = MockAccountDeletion {
            should_fail: false,
            should_not_delete: true,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            Box::new(&deletion as &dyn AccountDeletion),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
