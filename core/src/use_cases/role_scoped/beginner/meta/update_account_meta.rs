use crate::{
    domain::{
        dtos::{
            account::{AccountMeta, AccountMetaKey},
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{AccountUpdating, ResourceAuditLogRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "update_account_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, account_updating_repo, audit_repo)
)]
pub async fn update_account_meta(
    profile: Profile,
    key: AccountMetaKey,
    value: String,
    account_updating_repo: Box<&dyn AccountUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<AccountMeta>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    let response = account_updating_repo
        .update_account_meta(profile.acc_id, key.to_owned(), value)
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Emit the audit event
    // ? -----------------------------------------------------------------------

    if let UpdatingResponseKind::Updated(_) = &response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::AccountMeta,
            profile.acc_id,
            None,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "update_account_meta",
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
        dtos::{account::Account, account_type::AccountType},
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use mycelium_base::utils::errors::use_case_err;
    use std::collections::HashMap;
    use uuid::Uuid;

    struct MockAccountUpdating {
        should_fail: bool,
        should_not_update: bool,
    }

    #[async_trait]
    impl AccountUpdating for MockAccountUpdating {
        async fn update(
            &self,
            _: Account,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn update_own_account_name(
            &self,
            _: Uuid,
            _: String,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn update_account_type(
            &self,
            _: Uuid,
            _: AccountType,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn update_account_meta(
            &self,
            _: Uuid,
            _: AccountMetaKey,
            _: String,
        ) -> Result<UpdatingResponseKind<AccountMeta>, MappedErrors> {
            if self.should_fail {
                return use_case_err("simulated updating failure").as_error();
            }

            if self.should_not_update {
                return Ok(UpdatingResponseKind::NotUpdated(
                    HashMap::new(),
                    "not found".to_string(),
                ));
            }

            Ok(UpdatingResponseKind::Updated(HashMap::new()))
        }
    }

    #[tokio::test]
    async fn update_account_meta_emits_audit_event_on_success() {
        let profile = Profile::default();
        let account_id = profile.acc_id;

        let updating = MockAccountUpdating {
            should_fail: false,
            should_not_update: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::AccountMeta
                    && event.event == ResourceAuditEventKind::Updated
                    && event.resource_id == account_id
                    && event.tenant_id.is_none()
            })
            .returning(|_| Ok(()));

        let result = update_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            "+5511999999999".to_string(),
            Box::new(&updating as &dyn AccountUpdating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_account_meta_does_not_emit_audit_event_on_failure() {
        let profile = Profile::default();

        let updating = MockAccountUpdating {
            should_fail: true,
            should_not_update: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = update_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            "+5511999999999".to_string(),
            Box::new(&updating as &dyn AccountUpdating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_account_meta_does_not_emit_audit_event_when_not_updated() {
        let profile = Profile::default();

        let updating = MockAccountUpdating {
            should_fail: false,
            should_not_update: true,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = update_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            "+5511999999999".to_string(),
            Box::new(&updating as &dyn AccountUpdating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
