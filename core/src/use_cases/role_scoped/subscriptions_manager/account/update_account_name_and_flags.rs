use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::Account,
            account_type::AccountType,
            guest_role::Permission,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            webhook::{PayloadId, WebHookTrigger},
        },
        entities::{AccountFetching, AccountUpdating, WebHookRegistration},
    },
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "update_account_name_and_flags",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty
    ),
    skip_all
)]
pub async fn update_account_name_and_flags(
    profile: Profile,
    account_id: Uuid,
    tenant_id: Uuid,
    name: Option<String>,
    is_active: Option<bool>,
    is_checked: Option<bool>,
    is_archived: Option<bool>,
    is_system_account: Option<bool>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", &Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_accounts_or_tenant_wide_permission_or_error(
            tenant_id,
            Permission::Write,
        )?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    let mut account = match account_fetching_repo
        .get(account_id, related_accounts)
        .await?
    {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Account with id {} not found",
                id.unwrap_or_default()
            ))
            .as_error();
        }
    };

    if let Some(name) = name {
        account.name = name.to_owned();

        match account.account_type {
            AccountType::User | AccountType::Staff | AccountType::Manager => {
                return use_case_err(format!(
                    "Account type {} does not support name updating",
                    account.account_type.to_string()
                ))
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00018)
                .as_error();
            }
            _ => {}
        }

        //
        // If the account is not a role associated or tenant manager account,
        // then we can update the slug. In both cases, the slug should be
        // immutable.
        //
        if ![
            matches!(account.account_type, AccountType::RoleAssociated { .. }),
            matches!(account.account_type, AccountType::TenantManager { .. }),
        ]
        .iter()
        .all(|t| *t)
        {
            account.slug = slugify!(&name.as_str());
        }
    }

    if let Some(is_active) = is_active {
        account.is_active = is_active;
    }

    if let Some(is_checked) = is_checked {
        account.is_checked = is_checked;
    }

    if let Some(is_archived) = is_archived {
        account.is_archived = is_archived;
    }

    if let Some(is_system_account) = is_system_account {
        account.is_system_account = is_system_account;
    }

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    let response = account_updating_repo.update(account).await?;

    if let UpdatingResponseKind::Updated(account) = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        let account_id = account.id.ok_or_else(|| {
            use_case_err("Account ID not found".to_string()).with_exp_true()
        })?;

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::SubscriptionAccountUpdated,
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
    use crate::domain::{
        dtos::{
            account::{AccountMeta, AccountMetaKey},
            related_accounts::RelatedAccounts,
            webhook::{WebHook, WebHookPayloadArtifact},
        },
        entities::AccountFetching,
    };

    use async_trait::async_trait;
    use mycelium_base::entities::{CreateResponseKind, FetchManyResponseKind};

    fn account() -> Account {
        let mut account = Account::default();
        account.name = "Test Account".to_string();
        account.slug = slugify!(&account.name);
        account
    }

    struct MockAccountFetching {
        account: Account,
    }

    impl Default for MockAccountFetching {
        fn default() -> Self {
            Self { account: account() }
        }
    }

    impl MockAccountFetching {
        fn from_account(account: Account) -> Self {
            Self { account }
        }
    }

    #[async_trait]
    impl AccountFetching for MockAccountFetching {
        async fn get(
            &self,
            _: Uuid,
            _: RelatedAccounts,
        ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::Found(self.account.clone()))
        }

        async fn list(
            &self,
            _: RelatedAccounts,
            _: Option<String>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<Uuid>,
            _: Option<String>,
            _: Option<Uuid>,
            _: AccountType,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }
    }
    struct MockAccountUpdating {
        account: Account,
    }

    impl Default for MockAccountUpdating {
        fn default() -> Self {
            Self { account: account() }
        }
    }

    impl MockAccountUpdating {
        fn from_account(account: Account) -> Self {
            Self { account }
        }
    }

    #[async_trait]
    impl AccountUpdating for MockAccountUpdating {
        async fn update(
            &self,
            _: Account,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            Ok(UpdatingResponseKind::Updated(self.account.clone()))
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
            unimplemented!()
        }
    }

    struct MockWebHookRegistration {
        webhook: WebHook,
    }

    impl Default for MockWebHookRegistration {
        fn default() -> Self {
            Self {
                webhook: WebHook::new(
                    "Test WebHook".to_string(),
                    None,
                    "https://example.com".to_string(),
                    WebHookTrigger::SubscriptionAccountUpdated,
                    None,
                    None,
                ),
            }
        }
    }

    #[async_trait]
    impl WebHookRegistration for MockWebHookRegistration {
        async fn create(
            &self,
            _: WebHook,
        ) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
            Ok(CreateResponseKind::Created(self.webhook.clone()))
        }

        async fn register_execution_event(
            &self,
            _: WebHookPayloadArtifact,
        ) -> Result<CreateResponseKind<Uuid>, MappedErrors> {
            Ok(CreateResponseKind::Created(Uuid::new_v4()))
        }
    }

    ///
    /// Test updating the account name and flags
    ///
    /// Rules:
    /// - If it is an user, manager, or staff account, the slug field should
    ///   not be updated.
    /// - If it is a subscription account, the slug field should be updated.
    ///
    #[tokio::test]
    async fn test_update_account_name_and_flags() {
        let mut profile = Profile::default();
        profile.is_staff = true;

        let original_account = account();

        let account_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let new_name = original_account.name.clone();
        let is_active = Some(true);
        let is_checked = Some(true);
        let is_archived = Some(false);
        let is_default = Some(false);

        let fetching = MockAccountFetching::default();
        let updating = MockAccountUpdating::default();
        let webhook_registration = MockWebHookRegistration::default();

        let account_fetching_repo = Box::new(&fetching as &dyn AccountFetching);
        let account_updating_repo = Box::new(&updating as &dyn AccountUpdating);
        let webhook_registration_repo =
            Box::new(&webhook_registration as &dyn WebHookRegistration);

        let result = update_account_name_and_flags(
            profile,
            account_id,
            tenant_id,
            Some(new_name.clone()),
            is_active,
            is_checked,
            is_archived,
            is_default,
            account_fetching_repo,
            account_updating_repo,
            webhook_registration_repo,
        )
        .await;

        assert!(matches!(result, Err(_)));
        let error = result.unwrap_err();
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }

    async fn execute_update_account_name_and_flags(
        profile: Profile,
        old_account: Account,
        new_account: Account,
        new_name: String,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        let fetching = MockAccountFetching::from_account(old_account.clone());
        let updating = MockAccountUpdating::from_account(new_account.clone());
        let webhook_registration = MockWebHookRegistration::default();

        let account_fetching_repo = Box::new(&fetching as &dyn AccountFetching);
        let account_updating_repo = Box::new(&updating as &dyn AccountUpdating);
        let webhook_registration_repo =
            Box::new(&webhook_registration as &dyn WebHookRegistration);

        let result = update_account_name_and_flags(
            profile,
            old_account.id.unwrap(),
            Uuid::new_v4(),
            Some(new_name),
            Some(true),
            Some(true),
            Some(false),
            Some(false),
            account_fetching_repo,
            account_updating_repo,
            webhook_registration_repo,
        )
        .await;

        result
    }

    async fn assert_account_updated(
        result: Result<UpdatingResponseKind<Account>, MappedErrors>,
        expected_name: String,
        expected_slug: String,
    ) {
        let updated_account = match result {
            Ok(UpdatingResponseKind::Updated(account)) => account,
            _ => panic!("Expected an updated account"),
        };

        assert_eq!(updated_account.name, expected_name);
        assert_eq!(updated_account.slug, expected_slug);
    }

    ///
    ///  Test slug immutability
    ///
    #[tokio::test]
    async fn test_slug_immutability() {
        let mut profile = Profile::default();
        profile.is_staff = true;

        let old_name = "old-name".to_string();
        let new_name = "new-name".to_string();

        let mut subscription_account = Account::new_subscription_account(
            old_name.clone(),
            Uuid::new_v4(),
            None,
        );

        subscription_account.id = Some(Uuid::new_v4());

        let mut role_associated_account = Account::new_role_related_account(
            old_name.clone(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            old_name.clone(),
            false,
            None,
        );

        role_associated_account.id = Some(Uuid::new_v4());

        let subscription_account_result =
            execute_update_account_name_and_flags(
                profile.clone(),
                subscription_account.clone(),
                Account {
                    name: new_name.clone(),
                    slug: new_name.clone(),
                    ..subscription_account
                },
                new_name.clone(),
            )
            .await;

        let role_associated_account_result =
            execute_update_account_name_and_flags(
                profile.clone(),
                role_associated_account.clone(),
                Account {
                    name: new_name.clone(),
                    slug: old_name.clone(),
                    ..role_associated_account
                },
                new_name.clone(),
            )
            .await;

        assert_account_updated(
            subscription_account_result,
            new_name.clone(),
            new_name.clone(),
        )
        .await;

        assert_account_updated(
            role_associated_account_result,
            new_name.clone(),
            old_name.clone(),
        )
        .await;
    }
}
