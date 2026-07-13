mod register_token_and_notify_user;

use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            user::{PasswordHash, Provider, User},
            written_by::WrittenBy,
        },
        entities::{
            LocalMessageWrite, ResourceAuditLogRegistration, TenantFetching,
            TokenRegistration, UserDeletion, UserRegistration,
        },
    },
    models::AccountLifeCycle,
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use register_token_and_notify_user::register_token_and_notify_user;
use uuid::Uuid;

/// Create a new user with the default provider
///
/// This function creates a new user with the default provider. The default
/// provider is the internal provider, which uses the user's email and
/// password/provider to authenticate the user. Case the user is created with
/// the internal provider, the user is created as inactive, forcing the user to
/// confirm the email address before the user can use the system. The user
/// activation process is done by sending a confirmation email to the user.
/// If the user is created with an external provider, the user is created as
/// active.
///
#[tracing::instrument(name = "create_default_user", skip_all)]
pub async fn create_default_user(
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    provider_name: Option<String>,
    life_cycle_settings: AccountLifeCycle,
    user_registration_repo: Box<&dyn UserRegistration>,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    user_deletion_repo: Box<&dyn UserDeletion>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<Uuid, MappedErrors> {
    tracing::trace!("Creating user");

    // ? -----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the Email object, case an error is returned, the email is
    // possibly invalid.
    //
    // ? -----------------------------------------------------------------------

    let email_instance = Email::from_string(email.to_lowercase())?;

    // ? -----------------------------------------------------------------------
    // ? Build local user object
    // ? -----------------------------------------------------------------------

    if password.is_none() && provider_name.is_none() {
        return use_case_err(
            "At last one `password` or `provider-name` must contains a value"
                .to_string(),
        )
        .as_error();
    }

    let mut user = User::new_principal_with_provider(
        None,
        email_instance.to_owned(),
        match password.to_owned() {
            Some(password) => Provider::Internal(
                PasswordHash::hash_user_password(password.as_bytes()),
            ),
            None => Provider::External(provider_name.unwrap()),
        },
        first_name,
        last_name,
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the user
    //
    // New created user should be registered as inactive user (is_active =
    // false). The activation process should occur after the user confirm the
    // email address.
    //
    // ? -----------------------------------------------------------------------

    // ! By default new users are created as active ones. But when the user
    // ! provider is internal the user is created as inactive, forcing new users
    // ! to check their email address before they can use the system.
    if let Some(Provider::Internal(_)) = user.provider() {
        user.is_active = false;
    }

    let (new_user, was_created) = match user_registration_repo
        .get_or_create(user.to_owned())
        .await?
    {
        GetOrCreateResponseKind::NotCreated(user, _) => {
            if let Some(Provider::Internal(_)) = user.provider() {
                if user.is_active {
                    return use_case_err(
                        "You are trying to re-create an active user. Try to recovery your password instead"
                            .to_string(),
                    )
                    .with_code(NativeErrorCodes::MYC00002)
                    .with_exp_true()
                    .as_error();
                }

                (user, false)
            } else {
                (user, false)
            }
        }
        GetOrCreateResponseKind::Created(user) => (user, true),
    };

    let new_user_id = match new_user.id {
        None => {
            return use_case_err(
                "Unable to create user. Invalid user ID".to_string(),
            )
            .as_error()
        }
        Some(id) => id,
    };

    // ? -----------------------------------------------------------------------
    // ? Emit the audit event
    //
    // Only a real `Created` branch above represents a new row in the
    // database -- re-registering an existing inactive user does not.
    // ? -----------------------------------------------------------------------

    if was_created {
        emit_resource_audit_event(
            audit_repo.to_owned(),
            ResourceAuditResourceType::User,
            new_user_id,
            None,
            ResourceAuditEventKind::Created,
            WrittenBy::new_from_user_with_email(
                new_user_id,
                &email_instance.email(),
            ),
            serde_json::json!({ "action": "create_default_user" }),
        )
        .await;
    }

    // ? -----------------------------------------------------------------------
    // ? Notify internal user
    // ? -----------------------------------------------------------------------

    tracing::trace!("User Created. Dispatching side effects");

    if let Some(Provider::Internal(_)) = user.provider() {
        register_token_and_notify_user(
            new_user_id,
            email_instance.to_owned(),
            life_cycle_settings,
            token_registration_repo,
            message_sending_repo,
            user_deletion_repo,
            tenant_fetching_repo,
            audit_repo,
        )
        .await?;
    }

    tracing::trace!("Side effects dispatched");

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(new_user_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entities::MockResourceAuditLogRegistration,
        models::{HmacSecretEntry, HmacSecretSet},
    };

    use async_trait::async_trait;
    use chrono::Local;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::{CreateResponseKind, DeletionResponseKind};

    // ? -----------------------------------------------------------------------
    // ? Mock repositories
    // ? -----------------------------------------------------------------------

    struct MockUserRegistrationRepo;

    #[async_trait]
    impl UserRegistration for MockUserRegistrationRepo {
        async fn get_or_create(
            &self,
            user: User,
        ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
            let mut created = user;
            created.id = Some(Uuid::new_v4());

            Ok(GetOrCreateResponseKind::Created(created))
        }
    }

    struct UnimplementedTokenRegistrationRepo;

    #[async_trait]
    impl TokenRegistration for UnimplementedTokenRegistrationRepo {
        async fn create_email_confirmation_token(
            &self,
            _: crate::domain::dtos::token::EmailConfirmationTokenMeta,
            _: chrono::DateTime<Local>,
        ) -> Result<
            CreateResponseKind<crate::domain::dtos::token::Token>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn create_password_change_token(
            &self,
            _: crate::domain::dtos::token::PasswordChangeTokenMeta,
            _: chrono::DateTime<Local>,
        ) -> Result<
            CreateResponseKind<crate::domain::dtos::token::Token>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn create_connection_string(
            &self,
            _: crate::domain::dtos::token::UserAccountConnectionString,
            _: chrono::DateTime<Local>,
        ) -> Result<
            CreateResponseKind<crate::domain::dtos::token::Token>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn create_magic_link_token(
            &self,
            _: crate::domain::dtos::token::MagicLinkTokenMeta,
            _: chrono::DateTime<Local>,
        ) -> Result<
            CreateResponseKind<crate::domain::dtos::token::Token>,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    struct UnimplementedLocalMessageWriteRepo;

    #[async_trait]
    impl LocalMessageWrite for UnimplementedLocalMessageWriteRepo {
        async fn send(
            &self,
            _: crate::domain::dtos::message::MessageSendingEvent,
        ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
            unimplemented!()
        }

        async fn update_message_event(
            &self,
            _: crate::domain::dtos::message::MessageSendingEvent,
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

    struct UnimplementedUserDeletionRepo;

    #[async_trait]
    impl UserDeletion for UnimplementedUserDeletionRepo {
        async fn delete(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
        }
    }

    struct UnimplementedTenantFetchingRepo;

    #[async_trait]
    impl TenantFetching for UnimplementedTenantFetchingRepo {
        async fn get_tenant_owned_by_me(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<
            mycelium_base::entities::FetchResponseKind<
                crate::domain::dtos::tenant::Tenant,
                String,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn get_tenant_public_by_id(
            &self,
            _: Uuid,
        ) -> Result<
            mycelium_base::entities::FetchResponseKind<
                crate::domain::dtos::tenant::Tenant,
                String,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn get_tenants_by_manager_account(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<
            mycelium_base::entities::FetchResponseKind<
                crate::domain::dtos::tenant::Tenant,
                String,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn filter_tenants_as_manager(
            &self,
            _: Option<String>,
            _: Option<Uuid>,
            _: Option<(crate::domain::dtos::tenant::TenantMetaKey, String)>,
            _: Option<(String, String)>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<
                crate::domain::dtos::tenant::Tenant,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Helpers
    // ? -----------------------------------------------------------------------

    fn test_life_cycle_settings() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("Test Domain".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: None,
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

    // ? -----------------------------------------------------------------------
    // ? Test cases
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_default_user_emits_audit_event_on_success() {
        let user_registration_repo = MockUserRegistrationRepo;
        let token_registration_repo = UnimplementedTokenRegistrationRepo;
        let message_sending_repo = UnimplementedLocalMessageWriteRepo;
        let user_deletion_repo = UnimplementedUserDeletionRepo;
        let tenant_fetching_repo = UnimplementedTenantFetchingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::User
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        // ? External provider skips the confirmation-token/notification
        // ? side effects entirely, so this exercises the pure "created"
        // ? path without needing template rendering set up.
        let result = create_default_user(
            "test@example.com".to_string(),
            None,
            None,
            None,
            Some("google".to_string()),
            test_life_cycle_settings(),
            Box::new(&user_registration_repo),
            Box::new(&token_registration_repo),
            Box::new(&message_sending_repo),
            Box::new(&user_deletion_repo),
            Box::new(&tenant_fetching_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_default_user_does_not_emit_audit_event_on_validation_error()
    {
        let user_registration_repo = MockUserRegistrationRepo;
        let token_registration_repo = UnimplementedTokenRegistrationRepo;
        let message_sending_repo = UnimplementedLocalMessageWriteRepo;
        let user_deletion_repo = UnimplementedUserDeletionRepo;
        let tenant_fetching_repo = UnimplementedTenantFetchingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        // ? Neither `password` nor `provider_name` is set -- rejected before
        // ? any repository is touched.
        let result = create_default_user(
            "test@example.com".to_string(),
            None,
            None,
            None,
            None,
            test_life_cycle_settings(),
            Box::new(&user_registration_repo),
            Box::new(&token_registration_repo),
            Box::new(&message_sending_repo),
            Box::new(&user_deletion_repo),
            Box::new(&tenant_fetching_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
