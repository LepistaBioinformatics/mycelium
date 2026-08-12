use super::try_to_reach_desired_status::try_to_reach_desired_status;
use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountType,
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{
        AccountFetching, AccountUpdating, ResourceAuditLogRegistration,
    },
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Change activation status of the target account.
#[tracing::instrument(name = "change_account_activation_status", skip_all)]
pub async fn change_account_activation_status(
    profile: Profile,
    account_id: Uuid,
    is_active: bool,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::UsersManager])
        .get_related_account_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch target account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo
        .get(account_id, related_accounts)
        .await?
    {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Invalid account ID: {:?}", id))
                .as_error()
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Prevent self privilege escalation
    // ? -----------------------------------------------------------------------

    // Check if the account id os Some. Case false the operation is prohibited.
    let target_account_id = match account.id {
        None => {
            return use_case_err(format!(
                "Prohibited operation. Target account ({account_id}) could 
not be checked."
            ))
            .as_error()
        }
        Some(res) => res,
    };

    if target_account_id == profile.acc_id {
        return use_case_err(format!(
            "Prohibited operation. Account ID ({account_id}) could not be 
{target_account_id}."
        ))
        .as_error();
    }

    match account.to_owned().account_type {
        AccountType::Staff => {
            if profile.is_manager && !profile.is_staff {
                return use_case_err(String::from(
                    "Prohibited operation. Managers could not perform editions 
on accounts with more privileges than himself.",
                ))
                .as_error();
            }
        }
        _ => {
            return use_case_err(
                "Prohibited operation. Invalid account type".to_string(),
            )
            .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update account status
    // ? -----------------------------------------------------------------------

    let updated_account = try_to_reach_desired_status(
        account.to_owned(),
        match is_active {
            true => VerboseStatus::Verified,
            false => VerboseStatus::Inactive,
        },
    )
    .await?;

    let response = account_updating_repo.update(updated_account).await?;

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
            serde_json::json!({ "action": "change_account_activation_status" }),
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{account::AccountMetaKey, related_accounts::RelatedAccounts},
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use mycelium_base::entities::FetchManyResponseKind;
    use std::collections::HashMap;

    struct FakeAccountFetchingRepo;

    #[async_trait]
    impl AccountFetching for FakeAccountFetchingRepo {
        async fn get(
            &self,
            id: Uuid,
            _: RelatedAccounts,
        ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
            let mut account = Account::new_actor_related_account(
                "Account Name".to_string(),
                SystemActor::UsersManager,
                false,
                None,
            );
            account.id = Some(id);
            account.account_type = AccountType::Staff;

            Ok(FetchResponseKind::Found(account))
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

        async fn get_by_telegram_id(
            &self,
            _: crate::domain::dtos::telegram::TelegramUserId,
        ) -> Result<FetchResponseKind<Account, i64>, MappedErrors> {
            unimplemented!()
        }
    }

    struct FakeAccountUpdatingRepo;

    #[async_trait]
    impl AccountUpdating for FakeAccountUpdatingRepo {
        async fn update(
            &self,
            account: Account,
        ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
            Ok(UpdatingResponseKind::Updated(account))
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
    async fn change_account_activation_status_emits_audit_event_on_success() {
        let profile = staff_profile();
        let account_id = Uuid::new_v4();
        let fetching_repo = FakeAccountFetchingRepo;
        let updating_repo = FakeAccountUpdatingRepo;

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

        let result = change_account_activation_status(
            profile,
            account_id,
            true,
            Box::new(&fetching_repo),
            Box::new(&updating_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn change_account_activation_status_does_not_emit_audit_event_on_self_account(
    ) {
        let profile = staff_profile();
        let account_id = profile.acc_id;
        let fetching_repo = FakeAccountFetchingRepo;
        let updating_repo = FakeAccountUpdatingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = change_account_activation_status(
            profile,
            account_id,
            true,
            Box::new(&fetching_repo),
            Box::new(&updating_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn change_account_activation_status_does_not_emit_audit_event_on_permission_error(
    ) {
        let profile = Profile::default();
        let account_id = Uuid::new_v4();
        let fetching_repo = FakeAccountFetchingRepo;
        let updating_repo = FakeAccountUpdatingRepo;

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = change_account_activation_status(
            profile,
            account_id,
            true,
            Box::new(&fetching_repo),
            Box::new(&updating_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
