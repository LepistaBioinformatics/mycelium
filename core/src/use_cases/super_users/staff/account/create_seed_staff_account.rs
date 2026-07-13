use crate::domain::{
    dtos::{
        account::Account,
        account_type::AccountType,
        email::Email,
        native_error_codes::NativeErrorCodes,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        user::{PasswordHash, Provider, User},
        written_by::WrittenBy,
    },
    entities::{
        AccountRegistration, ResourceAuditLogRegistration, UserRegistration,
    },
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Create a seed staff account.
///
/// Seed staff accounts should be created over the system first initialization.
/// The seed staff will be create other users.
///
/// WARNING:
/// --------
///
/// Given the possibility to create seed staff accounts without profile
/// checking, this function could not be exposed through API ports.
#[tracing::instrument(name = "create_seed_staff_account", skip_all)]
pub async fn create_seed_staff_account(
    email: String,
    account_name: String,
    first_name: String,
    last_name: String,
    password: String,
    user_registration_repo: Box<&dyn UserRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the Email object, case an error is returned, the email is
    // possibly invalid.
    // ? -----------------------------------------------------------------------

    let email_instance = Email::from_string(email)?;

    // ? -----------------------------------------------------------------------
    // ? Build local user object
    // ? -----------------------------------------------------------------------

    let user = User::new_principal_with_provider(
        None,
        email_instance,
        Provider::Internal(PasswordHash::hash_user_password(
            password.as_bytes(),
        )),
        Some(first_name),
        Some(last_name),
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the user
    // ? -----------------------------------------------------------------------

    let new_user = match user_registration_repo
        .get_or_create(user.to_owned())
        .await?
    {
        GetOrCreateResponseKind::NotCreated(user, _) => {
            return use_case_err(format!(
                "User already registered: {}",
                user.email.email()
            ))
            .with_code(NativeErrorCodes::MYC00002)
            .as_error()
        }
        GetOrCreateResponseKind::Created(user) => user,
    };

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let new_user_id = new_user.id.ok_or_else(|| {
        use_case_err("User ID not found".to_string()).with_exp_true()
    })?;
    let performed_by = WrittenBy::new_from_user_with_email(
        new_user_id,
        &new_user.email.email(),
    );

    let response = account_registration_repo
        .get_or_create_user_account(
            Account::new(
                account_name,
                new_user.to_owned(),
                AccountType::Staff,
                Some(performed_by.to_owned()),
            ),
            true,
            false,
        )
        .await?;

    if let GetOrCreateResponseKind::Created(account) = &response {
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
            serde_json::json!({ "action": "create_seed_staff_account" }),
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;
    use uuid::Uuid;

    struct FakeUserRegistrationRepo;

    #[async_trait]
    impl UserRegistration for FakeUserRegistrationRepo {
        async fn get_or_create(
            &self,
            mut user: User,
        ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
            user.id = Some(Uuid::new_v4());
            Ok(GetOrCreateResponseKind::Created(user))
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

    #[tokio::test]
    async fn create_seed_staff_account_emits_audit_event_on_success() {
        let account_id = Uuid::new_v4();
        let user_registration_repo = FakeUserRegistrationRepo;
        let account_registration_repo =
            FakeAccountRegistrationRepo { account_id };

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

        let result = create_seed_staff_account(
            "seed@example.com".to_string(),
            "Seed Account".to_string(),
            "Seed".to_string(),
            "Staff".to_string(),
            "some-password".to_string(),
            Box::new(&user_registration_repo),
            Box::new(&account_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    struct FakeAlreadyRegisteredUserRegistrationRepo;

    #[async_trait]
    impl UserRegistration for FakeAlreadyRegisteredUserRegistrationRepo {
        async fn get_or_create(
            &self,
            user: User,
        ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
            Ok(GetOrCreateResponseKind::NotCreated(
                user,
                "already registered".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn create_seed_staff_account_does_not_emit_audit_event_when_user_already_exists(
    ) {
        let user_registration_repo = FakeAlreadyRegisteredUserRegistrationRepo;
        let account_registration_repo = FakeAccountRegistrationRepo {
            account_id: Uuid::new_v4(),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_seed_staff_account(
            "seed@example.com".to_string(),
            "Seed Account".to_string(),
            "Seed".to_string(),
            "Staff".to_string(),
            "some-password".to_string(),
            Box::new(&user_registration_repo),
            Box::new(&account_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
