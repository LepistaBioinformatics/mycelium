use std::collections::HashMap;

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
        entities::{AccountRegistration, ResourceAuditLogRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "create_account_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, account_registration_repo, audit_repo)
)]
pub async fn create_account_meta(
    profile: Profile,
    key: AccountMetaKey,
    value: String,
    account_registration_repo: Box<&dyn AccountRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    let response = account_registration_repo
        .register_account_meta(profile.acc_id, key.to_owned(), value)
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Emit the audit event
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::Created(_) = &response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::AccountMeta,
            profile.acc_id,
            None,
            ResourceAuditEventKind::Created,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "create_account_meta",
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
        dtos::account::Account, entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use mycelium_base::{
        entities::GetOrCreateResponseKind, utils::errors::use_case_err,
    };
    use uuid::Uuid;

    struct MockAccountRegistration {
        should_fail: bool,
        should_not_create: bool,
    }

    #[async_trait]
    impl AccountRegistration for MockAccountRegistration {
        async fn get_or_create_user_account(
            &self,
            _: Account,
            _: bool,
            _: bool,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn create_subscription_account(
            &self,
            _: Account,
            _: Uuid,
        ) -> Result<CreateResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn get_or_create_tenant_management_account(
            &self,
            _: Account,
            _: Uuid,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn get_or_create_role_related_account(
            &self,
            _: Account,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn get_or_create_actor_related_account(
            &self,
            _: Account,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn register_account_meta(
            &self,
            _: Uuid,
            _: AccountMetaKey,
            _: String,
        ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>
        {
            if self.should_fail {
                return use_case_err("simulated registration failure")
                    .as_error();
            }

            if self.should_not_create {
                return Ok(CreateResponseKind::NotCreated(
                    HashMap::new(),
                    "already exists".to_string(),
                ));
            }

            Ok(CreateResponseKind::Created(HashMap::new()))
        }
    }

    #[tokio::test]
    async fn create_account_meta_emits_audit_event_on_success() {
        let profile = Profile::default();
        let account_id = profile.acc_id;

        let registration = MockAccountRegistration {
            should_fail: false,
            should_not_create: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::AccountMeta
                    && event.event == ResourceAuditEventKind::Created
                    && event.resource_id == account_id
                    && event.tenant_id.is_none()
            })
            .returning(|_| Ok(()));

        let result = create_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            "+5511999999999".to_string(),
            Box::new(&registration as &dyn AccountRegistration),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_account_meta_does_not_emit_audit_event_on_failure() {
        let profile = Profile::default();

        let registration = MockAccountRegistration {
            should_fail: true,
            should_not_create: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            "+5511999999999".to_string(),
            Box::new(&registration as &dyn AccountRegistration),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_account_meta_does_not_emit_audit_event_when_not_created() {
        let profile = Profile::default();

        let registration = MockAccountRegistration {
            should_fail: false,
            should_not_create: true,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_account_meta(
            profile,
            AccountMetaKey::PhoneNumber,
            "+5511999999999".to_string(),
            Box::new(&registration as &dyn AccountRegistration),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
