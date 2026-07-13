use crate::domain::{
    dtos::{
        account::Account, account_type::AccountType,
        instance_settings::STAFF_BOOTSTRAP_KEY, user::User,
        written_by::WrittenBy,
    },
    entities::{AccountRegistration, InstanceSettingsRegistration},
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Claim the one-time staff bootstrap for `user`.
///
/// Claiming is inserting the `STAFF_BOOTSTRAP_KEY` row: whoever wins that
/// insert wins the claim (no separate CAS-update step needed now that row
/// presence *is* the claimed state). This still runs before the Account repo
/// is ever touched, so a lost race returns immediately without creating
/// anything (design.md D-1). Who claimed it and when are recorded via
/// `created_by`/`created` on the row itself -- no bespoke payload needed, so
/// `value` carries nothing but an empty marker.
#[tracing::instrument(name = "claim_staff_bootstrap", skip_all)]
pub async fn claim_staff_bootstrap(
    user: User,
    account_registration_repo: Box<&dyn AccountRegistration>,
    instance_settings_registration_repo: Box<&dyn InstanceSettingsRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    let user_id = user.id.ok_or_else(|| {
        use_case_err("User ID not found".to_string()).with_exp_true()
    })?;

    let claimed_by =
        WrittenBy::new_from_user_with_email(user_id, &user.email.email());

    match instance_settings_registration_repo
        .get_or_create(
            STAFF_BOOTSTRAP_KEY.to_string(),
            serde_json::json!({}),
            Some(claimed_by),
        )
        .await?
    {
        GetOrCreateResponseKind::NotCreated(..) => {
            return use_case_err("Staff bootstrap has already been completed")
                .with_exp_true()
                .as_error();
        }
        GetOrCreateResponseKind::Created(_) => {}
    };

    let account_name = user.email.email();

    let created_by =
        WrittenBy::new_from_user_with_email(user_id, &account_name);

    account_registration_repo
        .get_or_create_user_account(
            Account::new(
                account_name,
                user,
                AccountType::Staff,
                Some(created_by),
            ),
            true,
            false,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{dtos::account::AccountMetaKey, dtos::email::Email};

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::CreateResponseKind;
    use std::{
        collections::HashMap,
        sync::atomic::{AtomicBool, Ordering},
    };
    use uuid::Uuid;

    struct StubInstanceSettingsRegistration {
        response: GetOrCreateResponseKind<
            crate::domain::dtos::instance_settings::InstanceSetting,
        >,
    }

    #[async_trait]
    impl InstanceSettingsRegistration for StubInstanceSettingsRegistration {
        async fn get_or_create(
            &self,
            _: String,
            _: serde_json::Value,
            _: Option<WrittenBy>,
        ) -> Result<
            GetOrCreateResponseKind<
                crate::domain::dtos::instance_settings::InstanceSetting,
            >,
            MappedErrors,
        > {
            Ok(self.response.clone())
        }
    }

    #[derive(Default)]
    struct SpyAccountRegistration {
        called: AtomicBool,
    }

    #[async_trait]
    impl AccountRegistration for SpyAccountRegistration {
        async fn get_or_create_user_account(
            &self,
            account: Account,
            _: bool,
            _: bool,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            self.called.store(true, Ordering::SeqCst);
            Ok(GetOrCreateResponseKind::Created(account))
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
            unimplemented!()
        }
    }

    fn test_user() -> User {
        User::new(
            Some(Uuid::new_v4()),
            "test-staff".to_string(),
            Email::from_string("staff@example.com".to_string()).unwrap(),
            None,
            None,
            true,
            Local::now(),
            None,
            None,
            None,
        )
    }

    fn instance_setting(
        key: &str,
    ) -> crate::domain::dtos::instance_settings::InstanceSetting {
        crate::domain::dtos::instance_settings::InstanceSetting {
            key: key.to_string(),
            value: serde_json::json!({}),
            created_by: None,
            updated_by: None,
            created: Local::now(),
            updated: None,
        }
    }

    #[tokio::test]
    async fn creates_the_staff_account_when_the_claim_succeeds(
    ) -> Result<(), MappedErrors> {
        let registration_repo = StubInstanceSettingsRegistration {
            response: GetOrCreateResponseKind::Created(instance_setting(
                STAFF_BOOTSTRAP_KEY,
            )),
        };
        let account_repo = SpyAccountRegistration::default();

        let result = claim_staff_bootstrap(
            test_user(),
            Box::new(&account_repo as &dyn AccountRegistration),
            Box::new(&registration_repo as &dyn InstanceSettingsRegistration),
        )
        .await?;

        assert!(matches!(result, GetOrCreateResponseKind::Created(_)));
        assert!(account_repo.called.load(Ordering::SeqCst));

        Ok(())
    }

    #[tokio::test]
    async fn never_touches_the_account_repo_when_the_claim_is_lost() {
        let registration_repo = StubInstanceSettingsRegistration {
            response: GetOrCreateResponseKind::NotCreated(
                instance_setting(STAFF_BOOTSTRAP_KEY),
                "instance_settings row for key already existed".to_string(),
            ),
        };
        let account_repo = SpyAccountRegistration::default();

        let result = claim_staff_bootstrap(
            test_user(),
            Box::new(&account_repo as &dyn AccountRegistration),
            Box::new(&registration_repo as &dyn InstanceSettingsRegistration),
        )
        .await;

        assert!(result.is_err());
        assert!(!account_repo.called.load(Ordering::SeqCst));
    }
}
