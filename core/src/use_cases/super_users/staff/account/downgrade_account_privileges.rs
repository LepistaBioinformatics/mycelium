use crate::domain::{
    dtos::{
        account::Account,
        account_type::AccountType,
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{AccountUpdating, ResourceAuditLogRegistration},
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

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
    skip(profile, account_updating_repo, audit_repo)
)]
pub async fn downgrade_account_privileges(
    profile: Profile,
    account_id: Uuid,
    target_account_type: AccountType,
    account_updating_repo: Box<&dyn AccountUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
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

    let response = account_updating_repo
        .update_account_type(account_id, target_account_type)
        .await?;

    if let UpdatingResponseKind::Updated(account) = &response {
        let tenant_id = match &account.account_type {
            AccountType::Subscription { tenant_id } => Some(*tenant_id),
            AccountType::RoleAssociated { tenant_id, .. } => Some(*tenant_id),
            AccountType::TenantManager { tenant_id } => Some(*tenant_id),
            _ => None,
        };

        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Account,
            account_id,
            tenant_id,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "downgrade_account_privileges" }),
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::account::{AccountMeta, AccountMetaKey};
    use crate::domain::dtos::account_type::AccountType;
    use crate::domain::dtos::profile::Profile;
    use crate::domain::entities::MockResourceAuditLogRegistration;

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

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.event == ResourceAuditEventKind::Updated
            })
            .returning(|_| Ok(()));

        let result = downgrade_account_privileges(
            profile,
            account_id,
            target_account_type,
            account_updating_repo,
            Box::new(&audit_mock),
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

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.event == ResourceAuditEventKind::Updated
            })
            .returning(|_| Ok(()));

        let result = downgrade_account_privileges(
            profile,
            account_id,
            target_account_type,
            account_updating_repo,
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_downgrade_account_privileges_does_not_emit_audit_event_on_permission_error(
    ) {
        let profile = Profile::default();

        let account_id = Uuid::new_v4();
        let target_account_type = AccountType::User;

        let updating = MockAccountUpdating::default();
        let account_updating_repo = Box::new(&updating as &dyn AccountUpdating);

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = downgrade_account_privileges(
            profile,
            account_id,
            target_account_type,
            account_updating_repo,
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
