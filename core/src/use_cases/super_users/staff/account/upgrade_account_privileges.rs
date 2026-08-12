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

/// Upgrade the account status.
///
/// This action should be used to upgrade Standard, Manager, and Staff accounts.
/// Subscription accounts should not be upgraded.
#[tracing::instrument(
    name = "upgrade_account_privileges", 
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, account_updating_repo, audit_repo)
)]
pub async fn upgrade_account_privileges(
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

    if !profile.is_staff {
        return use_case_err(
            "The current user has no sufficient privileges to upgrade accounts.",
        )
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00018)
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Check if the account type if allowed
    // ? -----------------------------------------------------------------------

    if ![AccountType::Manager, AccountType::Staff]
        .contains(&target_account_type)
    {
        return use_case_err("Invalid upgrade target.")
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
            serde_json::json!({ "action": "upgrade_account_privileges" }),
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::account::AccountMetaKey,
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use std::collections::HashMap;

    struct FakeAccountUpdatingRepo;

    #[async_trait]
    impl AccountUpdating for FakeAccountUpdatingRepo {
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

        async fn update_account_type(
            &self,
            account_id: Uuid,
            account_type: AccountType,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            let account = Account::new_actor_related_account(
                "Account Name".to_string(),
                crate::domain::actors::SystemActor::SystemManager,
                false,
                None,
            );

            let mut account = account;
            account.id = Some(account_id);
            account.account_type = account_type;

            Ok(UpdatingResponseKind::Updated(account))
        }

        async fn update_account_meta(
            &self,
            _: Uuid,
            _: AccountMetaKey,
            _: String,
        ) -> Result<
            UpdatingResponseKind<HashMap<AccountMetaKey, String>>,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    fn staff_profile() -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            true,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn upgrade_account_privileges_emits_audit_event_on_success() {
        let profile = staff_profile();
        let account_id = Uuid::new_v4();
        let repo = FakeAccountUpdatingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Updated
            })
            .returning(|_| Ok(()));

        let result = upgrade_account_privileges(
            profile,
            account_id,
            AccountType::Manager,
            Box::new(&repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn upgrade_account_privileges_does_not_emit_audit_event_on_permission_error(
    ) {
        let profile = Profile::default();
        let account_id = Uuid::new_v4();
        let repo = FakeAccountUpdatingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = upgrade_account_privileges(
            profile,
            account_id,
            AccountType::Manager,
            Box::new(&repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn upgrade_account_privileges_does_not_emit_audit_event_on_invalid_target(
    ) {
        let profile = staff_profile();
        let account_id = Uuid::new_v4();
        let repo = FakeAccountUpdatingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = upgrade_account_privileges(
            profile,
            account_id,
            AccountType::User,
            Box::new(&repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
