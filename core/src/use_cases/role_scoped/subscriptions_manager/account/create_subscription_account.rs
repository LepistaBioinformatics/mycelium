use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::Account,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            webhook::{PayloadId, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{
            AccountRegistration, ResourceAuditLogRegistration,
            WebHookRegistration,
        },
    },
    use_cases::shared::audit::emit_resource_audit_event,
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::Instrument;
use uuid::Uuid;

/// Create an account flagged as subscription.
///
/// Subscription accounts represents results centering accounts.
#[tracing::instrument(
    name = "create_subscription_account",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty,
    ),
    skip_all
)]
pub async fn create_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_name: String,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    let span = tracing::Span::current();
    span.record("correspondence_id", Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a subscription account");

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let is_owner = profile.with_tenant_ownership_or_error(tenant_id).is_ok();

    let has_access = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()
        .is_ok();

    if ![is_owner, has_access].iter().any(|&x| x) {
        return use_case_err(
            "Insufficient privileges to create a subscription account",
        )
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let performed_by = WrittenBy::new_from_account(profile.acc_id);

    let mut unchecked_account = Account::new_subscription_account(
        account_name,
        tenant_id,
        Some(performed_by.to_owned()),
    );

    unchecked_account.is_checked = true;

    let account = match account_registration_repo
        .create_subscription_account(unchecked_account, tenant_id)
        .await?
    {
        CreateResponseKind::NotCreated(account, msg) => {
            return use_case_err(format!("({}): {}", account.name, msg))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        }
        CreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    tracing::trace!("Dispatching side effects");

    let account_id = account.id.ok_or_else(|| {
        use_case_err("Account ID not found".to_string()).with_exp_true()
    })?;

    emit_resource_audit_event(
        audit_repo,
        ResourceAuditResourceType::Account,
        account_id,
        Some(tenant_id),
        ResourceAuditEventKind::Created,
        performed_by,
        serde_json::json!({ "action": "create_subscription_account" }),
    )
    .await;

    register_webhook_dispatching_event(
        correspondence_id,
        WebHookTrigger::SubscriptionAccountCreated,
        account.to_owned(),
        PayloadId::Uuid(account_id),
        webhook_registration_repo,
    )
    .instrument(span)
    .await?;

    tracing::trace!("Side effects dispatched");

    Ok(account)
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
    use mycelium_base::entities::GetOrCreateResponseKind;
    use std::collections::HashMap;

    struct FakeAccountRegistrationRepo {
        account_id: Uuid,
    }

    #[async_trait]
    impl AccountRegistration for FakeAccountRegistrationRepo {
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
            mut account: Account,
            _: Uuid,
        ) -> Result<CreateResponseKind<Account>, MappedErrors> {
            account.id = Some(self.account_id);
            Ok(CreateResponseKind::Created(account))
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
            _: crate::domain::dtos::account::AccountMetaKey,
            _: String,
        ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>
        {
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
    async fn create_subscription_account_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let registration_repo = FakeAccountRegistrationRepo { account_id };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        let result = create_subscription_account(
            profile,
            tenant_id,
            "Subscription Account".to_string(),
            Box::new(&registration_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_subscription_account_does_not_emit_audit_event_on_permission_error(
    ) {
        let tenant_id = Uuid::new_v4();
        let profile = Profile::default();
        let registration_repo = FakeAccountRegistrationRepo {
            account_id: Uuid::new_v4(),
        };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_subscription_account(
            profile,
            tenant_id,
            "Subscription Account".to_string(),
            Box::new(&registration_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
