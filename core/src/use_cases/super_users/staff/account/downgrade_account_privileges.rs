use crate::domain::{
    dtos::{
        account::Account, account_type::AccountType,
        native_error_codes::NativeErrorCodes, profile::Profile,
    },
    entities::AccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Downgrade the account status.
///
/// This action should be used to downgrade Standard and Manager accounts.
/// Subscription and Staff accounts should not be downgraded.
#[tracing::instrument(
    name = "downgrade_account_privileges", 
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, account_updating_repo)
)]
pub async fn downgrade_account_privileges(
    profile: Profile,
    account_id: Uuid,
    target_account_type: AccountType,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    //
    // Only staff users should perform such action.
    // ? -----------------------------------------------------------------------

    if ![
        //
        // Only staff users should perform such action in other accounts.
        //
        profile.is_staff,
        //
        // The account to downgrade should be the current account.
        //
        account_id == profile.acc_id,
    ]
    .iter()
    .any(|&b| b)
    {
        return use_case_err(
            "The current user has no sufficient privileges to downgrade accounts.",
        )
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00019)
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Check if the account type if allowed
    // ? -----------------------------------------------------------------------

    if ![AccountType::User, AccountType::Manager].contains(&target_account_type)
    {
        return use_case_err("Invalid downgrade target.")
            .with_exp_true()
            .with_code(NativeErrorCodes::MYC00018)
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    account_updating_repo
        .update_account_type(account_id, target_account_type)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::account::{AccountMeta, AccountMetaKey};
    use crate::domain::dtos::account_type::AccountType;
    use crate::domain::dtos::profile::Profile;

    use async_trait::async_trait;

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
            unimplemented!()
        }

        async fn update_own_account_name(
            &self,
            _: Uuid,
            _: String,
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

        async fn update_account_type(
            &self,
            _: Uuid,
            _: AccountType,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            Ok(UpdatingResponseKind::Updated(Account::default()))
        }
    }

    #[tokio::test]
    async fn test_downgrade_account_privileges_with_non_self_account() {
        let mut profile = Profile::default();
        profile.is_staff = true;

        let account_id = Uuid::new_v4();
        let target_account_type = AccountType::User;

        let updating = MockAccountUpdating::default();
        let account_updating_repo = Box::new(&updating as &dyn AccountUpdating);

        let result = downgrade_account_privileges(
            profile,
            account_id,
            target_account_type,
            account_updating_repo,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_downgrade_account_privileges_with_self_account() {
        let mut profile = Profile::default();
        profile.is_staff = true;

        let account_id = profile.acc_id;
        let target_account_type = AccountType::User;

        let updating = MockAccountUpdating::default();
        let account_updating_repo = Box::new(&updating as &dyn AccountUpdating);

        let result = downgrade_account_privileges(
            profile,
            account_id,
            target_account_type,
            account_updating_repo,
        )
        .await;

        assert!(result.is_ok());
    }
}
