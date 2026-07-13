use crate::{
    domain::{
        dtos::{
            account::Account,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            webhook::{PayloadId, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{
            AccountUpdating, ResourceAuditLogRegistration, WebHookRegistration,
        },
    },
    use_cases::shared::audit::emit_resource_audit_event,
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::Instrument;
use uuid::Uuid;

/// Update the own account.
///
/// This function uses the id of the Profile to fetch and update the account
/// name, allowing only the account owner to update the account name.
#[tracing::instrument(
    name = "update_own_account_name", 
    skip_all,
    fields(correspondence_id = tracing::field::Empty),
)]
pub async fn update_own_account_name(
    profile: Profile,
    name: String,
    account_updating_repo: Box<&dyn AccountUpdating>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    let response = account_updating_repo
        .update_own_account_name(profile.acc_id, name)
        .await?;

    if let UpdatingResponseKind::Updated(account) = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        let account_id = account.id.ok_or_else(|| {
            use_case_err("Account ID not found".to_string()).with_exp_true()
        })?;

        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Account,
            account_id,
            None,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "update_own_account_name" }),
        )
        .await;

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountUpdated,
            account.to_owned(),
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
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;
    use mycelium_base::entities::CreateResponseKind;
    use std::collections::HashMap;

    struct FakeAccountUpdatingRepo;

    #[async_trait]
    impl AccountUpdating for FakeAccountUpdatingRepo {
        async fn update(
            &self,
            _: Account,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn update_own_account_name(
            &self,
            account_id: Uuid,
            name: String,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            let mut account = Account::default();
            account.id = Some(account_id);
            account.name = name;

            Ok(UpdatingResponseKind::Updated(account))
        }

        async fn update_account_type(
            &self,
            _: Uuid,
            _: crate::domain::dtos::account_type::AccountType,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn update_account_meta(
            &self,
            _: Uuid,
            _: crate::domain::dtos::account::AccountMetaKey,
            _: String,
        ) -> Result<
            UpdatingResponseKind<
                HashMap<crate::domain::dtos::account::AccountMetaKey, String>,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    struct FakeAccountUpdatingRepoNotUpdated;

    #[async_trait]
    impl AccountUpdating for FakeAccountUpdatingRepoNotUpdated {
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
            Ok(UpdatingResponseKind::NotUpdated(
                Account::default(),
                "not updated".to_string(),
            ))
        }

        async fn update_account_type(
            &self,
            _: Uuid,
            _: crate::domain::dtos::account_type::AccountType,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn update_account_meta(
            &self,
            _: Uuid,
            _: crate::domain::dtos::account::AccountMetaKey,
            _: String,
        ) -> Result<
            UpdatingResponseKind<
                HashMap<crate::domain::dtos::account::AccountMetaKey, String>,
            >,
            MappedErrors,
        > {
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
    async fn update_own_account_name_emits_audit_event_on_success() {
        let profile = Profile::default();
        let account_id = profile.acc_id;
        let account_updating_repo = FakeAccountUpdatingRepo;
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Updated
            })
            .returning(|_| Ok(()));

        let result = update_own_account_name(
            profile,
            "New Name".to_string(),
            Box::new(&account_updating_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_own_account_name_does_not_emit_audit_event_when_not_updated(
    ) {
        let profile = Profile::default();
        let account_updating_repo = FakeAccountUpdatingRepoNotUpdated;
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = update_own_account_name(
            profile,
            "New Name".to_string(),
            Box::new(&account_updating_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
