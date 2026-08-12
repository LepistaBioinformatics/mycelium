use crate::{
    domain::{
        dtos::{
            account::Account,
            account_type::AccountType,
            email::Email,
            native_error_codes::NativeErrorCodes,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            user::{Provider, User},
            webhook::{PayloadId, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{
            AccountRegistration, LocalMessageWrite,
            ResourceAuditLogRegistration, TenantFetching, UserFetching,
            UserRegistration, WebHookRegistration,
        },
    },
    models::AccountLifeCycle,
    use_cases::shared::audit::emit_resource_audit_event,
    use_cases::support::{
        dispatch_notification, register_webhook_dispatching_event,
    },
};

use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
use uuid::Uuid;

/// Create a default account.
///
/// Default accounts are used to mirror human users. Such accounts should not be
/// flagged as `subscription`.
///
/// This function are called when a new user start into the system. The
/// account-creation method also insert a new user into the database and set the
/// default role as `default-user`.
#[tracing::instrument(
    name = "create_user_account",
    fields(correspondence_id = tracing::field::Empty),
    skip_all
)]
pub async fn create_user_account(
    email: Email,
    provider: Option<String>,
    account_name: String,
    config: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a user account");

    // ? -----------------------------------------------------------------------
    // ? Try to fetch user from database
    // ? -----------------------------------------------------------------------

    let (identity_is_verified, _provider) = if let Some(provider) = provider {
        (true, Some(provider))
    } else {
        (false, None)
    };

    let user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            if !identity_is_verified || _provider.is_none() {
                return use_case_err("User not found".to_string()).as_error();
            }

            register_user_with_provider(
                email.to_owned(),
                _provider.unwrap(),
                user_registration_repo,
            )
            .await?
        }
        FetchResponseKind::Found(user) => user,
    };

    if let Some(Provider::Internal(_)) = user.provider() {
        if !user.is_active {
            return use_case_err("User is not active".to_string())
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00018)
                .as_error();
        }
    }

    if !user.is_principal() {
        return use_case_err("User is not the principal".to_string())
            .with_exp_true()
            .with_code(NativeErrorCodes::MYC00018)
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let user_id = user.id.ok_or_else(|| {
        use_case_err("User ID not found".to_string()).with_exp_true()
    })?;
    let performed_by =
        WrittenBy::new_from_user_with_email(user_id, &user.email.email());

    let mut base_account = Account::new(
        account_name.to_owned(),
        user.clone(),
        AccountType::User,
        Some(performed_by.to_owned()),
    );

    base_account.slug = slugify!(user.email.email().as_str());

    let account = match account_registration_repo
        .get_or_create_user_account(base_account, true, false)
        .await?
    {
        GetOrCreateResponseKind::Created(account) => account,
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(format!("Account not created: {msg}"))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Perform finishing operations
    // ? -----------------------------------------------------------------------

    tracing::trace!("Dispatching side effects");

    let account_id = account.id.ok_or_else(|| {
        use_case_err("Account ID not found".to_string()).with_exp_true()
    })?;

    emit_resource_audit_event(
        audit_repo,
        ResourceAuditResourceType::Account,
        account_id,
        None,
        ResourceAuditEventKind::Created,
        performed_by,
        serde_json::json!({ "action": "create_user_account" }),
    )
    .await;

    let (notification_response, webhook_responses) = futures::join!(
        dispatch_notification(
            vec![("account_name", account_name)],
            "email/create-user-account",
            config.to_owned(),
            email,
            None,
            message_sending_repo,
            tenant_fetching_repo,
        ),
        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountCreated,
            account.to_owned(),
            PayloadId::Uuid(account_id),
            webhook_registration_repo,
        )
    );

    if let Err(err) = notification_response {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    }

    if let Err(err) = webhook_responses {
        return use_case_err(format!("Unable to register webhook: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    tracing::trace!("Side effects dispatched");

    // ? -----------------------------------------------------------------------
    // ? Return the webhook responses
    // ? -----------------------------------------------------------------------

    Ok(account)
}

async fn register_user_with_provider(
    email: Email,
    provider: String,
    user_registration_repo: Box<&dyn UserRegistration>,
) -> Result<User, MappedErrors> {
    let user = User::new_principal_with_provider(
        None,
        email.to_owned(),
        Provider::External(provider),
        None,
        None,
    )?;

    match user_registration_repo
        .get_or_create(user.to_owned())
        .await?
    {
        GetOrCreateResponseKind::Created(user) => Ok(user),
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            tracing::error!("User not created: {msg}");

            use_case_err("User not created".to_string()).as_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            message::MessageSendingEvent,
            tenant::{Tenant, TenantMetaKey},
            user::Provider,
            webhook::WebHookPayloadArtifact,
        },
        entities::MockResourceAuditLogRegistration,
    };
    use crate::models::{HmacSecretEntry, HmacSecretSet};

    use async_trait::async_trait;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::{FetchManyResponseKind, FetchResponseKind};
    use std::env;

    fn setup_templates_dir() {
        if env::var("TEMPLATES_DIR").is_err() {
            if let Ok(current_dir) = env::current_dir() {
                let templates_path = current_dir.join("templates");
                if let Some(abs_path) = templates_path.to_str() {
                    env::set_var("TEMPLATES_DIR", abs_path);
                }
            }
        }
    }

    fn create_test_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("Test Domain".to_string()),
            domain_url: Some(SecretResolver::Value(
                "https://test.com".to_string(),
            )),
            locale: Some(SecretResolver::Value("en-us".to_string())),
            token_expiration: SecretResolver::Value(3600),
            noreply_name: Some(SecretResolver::Value(
                "Test System".to_string(),
            )),
            noreply_email: SecretResolver::Value(
                "noreply@test.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@test.com".to_string(),
            ),
            token_secret: SecretResolver::Value("test-secret".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("test-hmac".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    struct FakeUserFetchingRepo;

    #[async_trait]
    impl UserFetching for FakeUserFetchingRepo {
        async fn get_user_by_email(
            &self,
            email: Email,
        ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
            let user = User::new_principal_with_provider(
                None,
                email,
                Provider::External("test".to_string()),
                Some("First".to_string()),
                Some("Last".to_string()),
            )?;
            let mut user = user;
            user.id = Some(Uuid::new_v4());

            Ok(FetchResponseKind::Found(user))
        }

        async fn get_user_by_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_not_redacted_user_by_email(
            &self,
            _: Email,
        ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
            unimplemented!()
        }
    }

    struct FakeUserRegistrationRepo;

    #[async_trait]
    impl UserRegistration for FakeUserRegistrationRepo {
        async fn get_or_create(
            &self,
            _: User,
        ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
            unimplemented!()
        }
    }

    struct FakeAccountRegistrationRepo {
        account_id: Uuid,
    }

    #[async_trait]
    impl AccountRegistration for FakeAccountRegistrationRepo {
        async fn get_or_create_user_account(
            &self,
            mut account: Account,
            _: bool,
            _: bool,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            account.id = Some(self.account_id);
            Ok(GetOrCreateResponseKind::Created(account))
        }

        async fn create_subscription_account(
            &self,
            _: Account,
            _: Uuid,
        ) -> Result<
            mycelium_base::entities::CreateResponseKind<Account>,
            MappedErrors,
        > {
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
            _: crate::domain::dtos::account::AccountMetaKey,
            _: String,
        ) -> Result<
            mycelium_base::entities::CreateResponseKind<
                std::collections::HashMap<String, String>,
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
            mycelium_base::entities::CreateResponseKind<
                crate::domain::dtos::webhook::WebHook,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn register_execution_event(
            &self,
            _: WebHookPayloadArtifact,
        ) -> Result<
            mycelium_base::entities::CreateResponseKind<Uuid>,
            MappedErrors,
        > {
            Ok(mycelium_base::entities::CreateResponseKind::Created(
                Uuid::new_v4(),
            ))
        }
    }

    struct FakeLocalMessageWriteRepo;

    #[async_trait]
    impl LocalMessageWrite for FakeLocalMessageWriteRepo {
        async fn send(
            &self,
            _: MessageSendingEvent,
        ) -> Result<
            mycelium_base::entities::CreateResponseKind<Option<Uuid>>,
            MappedErrors,
        > {
            Ok(mycelium_base::entities::CreateResponseKind::Created(Some(
                Uuid::new_v4(),
            )))
        }

        async fn update_message_event(
            &self,
            _: MessageSendingEvent,
        ) -> Result<(), MappedErrors> {
            unimplemented!()
        }

        async fn delete_message_event(
            &self,
            _: Uuid,
        ) -> Result<(), MappedErrors> {
            unimplemented!()
        }

        async fn ping(&self) -> Result<(), MappedErrors> {
            unimplemented!()
        }
    }

    struct FakeTenantFetchingRepo;

    #[async_trait]
    impl TenantFetching for FakeTenantFetchingRepo {
        async fn get_tenant_owned_by_me(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_tenant_public_by_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_tenants_by_manager_account(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn filter_tenants_as_manager(
            &self,
            _: Option<String>,
            _: Option<Uuid>,
            _: Option<(TenantMetaKey, String)>,
            _: Option<(String, String)>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn create_user_account_emits_audit_event_on_success() {
        setup_templates_dir();

        let account_id = Uuid::new_v4();
        let email = Email::from_string("user@example.com".to_string())
            .expect("valid email");

        let user_fetching_repo = FakeUserFetchingRepo;
        let user_registration_repo = FakeUserRegistrationRepo;
        let account_registration_repo =
            FakeAccountRegistrationRepo { account_id };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;
        let message_sending_repo = FakeLocalMessageWriteRepo;
        let tenant_fetching_repo = FakeTenantFetchingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        let result = create_user_account(
            email,
            None,
            "Account Name".to_string(),
            create_test_config(),
            Box::new(&user_fetching_repo),
            Box::new(&user_registration_repo),
            Box::new(&account_registration_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&message_sending_repo),
            Box::new(&tenant_fetching_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    struct FakeUserNotFoundFetchingRepo;

    #[async_trait]
    impl UserFetching for FakeUserNotFoundFetchingRepo {
        async fn get_user_by_email(
            &self,
            _: Email,
        ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
            Ok(FetchResponseKind::NotFound(None))
        }

        async fn get_user_by_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_not_redacted_user_by_email(
            &self,
            _: Email,
        ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn create_user_account_does_not_emit_audit_event_when_user_not_found()
    {
        setup_templates_dir();

        let account_id = Uuid::new_v4();
        let email = Email::from_string("user@example.com".to_string())
            .expect("valid email");

        let user_fetching_repo = FakeUserNotFoundFetchingRepo;
        let user_registration_repo = FakeUserRegistrationRepo;
        let account_registration_repo =
            FakeAccountRegistrationRepo { account_id };
        let webhook_registration_repo = FakeWebHookRegistrationRepo;
        let message_sending_repo = FakeLocalMessageWriteRepo;
        let tenant_fetching_repo = FakeTenantFetchingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_user_account(
            email,
            None,
            "Account Name".to_string(),
            create_test_config(),
            Box::new(&user_fetching_repo),
            Box::new(&user_registration_repo),
            Box::new(&account_registration_repo),
            Box::new(&webhook_registration_repo),
            Box::new(&message_sending_repo),
            Box::new(&tenant_fetching_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
