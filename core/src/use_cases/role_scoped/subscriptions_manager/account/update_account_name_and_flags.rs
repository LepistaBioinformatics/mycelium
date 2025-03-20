use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account, account_type::AccountType,
        native_error_codes::NativeErrorCodes, profile::Profile,
    },
    entities::{AccountFetching, AccountUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
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
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_read_write_access()
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

    account_updating_repo.update(account).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            account::{AccountMeta, AccountMetaKey},
            related_accounts::RelatedAccounts,
        },
        entities::AccountFetching,
    };

    use async_trait::async_trait;
    use mycelium_base::entities::FetchManyResponseKind;

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

        let account_fetching_repo = Box::new(&fetching as &dyn AccountFetching);
        let account_updating_repo = Box::new(&updating as &dyn AccountUpdating);

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
        )
        .await;

        assert!(matches!(result, Err(_)));
        let error = result.unwrap_err();
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }
}
