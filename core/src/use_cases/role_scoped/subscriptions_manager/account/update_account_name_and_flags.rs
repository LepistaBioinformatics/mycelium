use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::Account,
            account_type::AccountType,
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
    fields(profile_id = %profile.acc_id),
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
    is_default: Option<bool>,
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
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()?;

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

        account.slug = slugify!(&name.as_str());
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

    if let Some(is_default) = is_default {
        account.is_default = is_default;
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

    struct MockAccountFetching;

    impl Default for MockAccountFetching {
        fn default() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl AccountFetching for MockAccountFetching {
        async fn get(
            &self,
            _: Uuid,
            _: RelatedAccounts,
        ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::Found(account()))
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
    struct MockAccountUpdating;

    impl Default for MockAccountUpdating {
        fn default() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl AccountUpdating for MockAccountUpdating {
        async fn update(
            &self,
            _: Account,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            Ok(UpdatingResponseKind::Updated(Account::default()))
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

    struct MockWebHookRegistration;

    impl Default for MockWebHookRegistration {
        fn default() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl WebHookRegistration for MockWebHookRegistration {
        async fn create(
            &self,
            _: WebHook,
        ) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
            unimplemented!()
        }

        async fn register_execution_event(
            &self,
            _: WebHookPayloadArtifact,
        ) -> Result<CreateResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
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
}
