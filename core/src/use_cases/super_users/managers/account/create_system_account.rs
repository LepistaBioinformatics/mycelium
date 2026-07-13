use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account,
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{AccountRegistration, ResourceAuditLogRegistration},
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Create a system account
///
/// System accounts should be used to guest users to system roles. Only system
/// accounts. System accounts are privileged accounts destined to group users
/// with specific roles, such as:
///
/// - Guest Managers
/// - System Managers
/// - Gateway Managers
///
#[tracing::instrument(
    name = "create_system_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, account_registration_repo, audit_repo)
)]
pub async fn create_system_account(
    profile: Profile,
    name: String,
    actor: SystemActor,
    account_registration_repo: Box<&dyn AccountRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Check if the desired actor should be created here
    // ? -----------------------------------------------------------------------

    let allowed_actors = [
        SystemActor::GatewayManager,
        SystemActor::GuestsManager,
        SystemActor::SystemManager,
    ];

    if !allowed_actors.contains(&actor) {
        return use_case_err(format!(
            "Only system actors accounts should be created. Given: {}",
            allowed_actors
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .with_code(NativeErrorCodes::MYC00013)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Create and register account
    // ? -----------------------------------------------------------------------

    let performed_by = WrittenBy::new_from_account(profile.acc_id);

    let mut unchecked_account = Account::new_actor_related_account(
        name,
        actor,
        true,
        Some(performed_by.to_owned()),
    );

    unchecked_account.is_checked = true;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    let response = account_registration_repo
        .get_or_create_actor_related_account(unchecked_account)
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
            serde_json::json!({ "action": "create_system_account" }),
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
    use mycelium_base::entities::CreateResponseKind;
    use std::collections::HashMap;
    use uuid::Uuid;

    struct FakeAccountRegistrationRepo {
        account_id: Uuid,
    }

    #[async_trait]
    impl AccountRegistration for FakeAccountRegistrationRepo {
        async fn get_or_create_user_account(
            &self,
            _: Account,
            _: bool,
            _: bool,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            unimplemented!()
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
            mut account: Account,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            account.id = Some(self.account_id);
            Ok(GetOrCreateResponseKind::Created(account))
        }

        async fn register_account_meta(
            &self,
            _: Uuid,
            _: crate::domain::dtos::account::AccountMetaKey,
            _: String,
        ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>
        {
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
    async fn create_system_account_emits_audit_event_on_success() {
        let profile = staff_profile();
        let account_id = Uuid::new_v4();
        let registration_repo = FakeAccountRegistrationRepo { account_id };

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

        let result = create_system_account(
            profile,
            "system-account".to_string(),
            SystemActor::SystemManager,
            Box::new(&registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_system_account_does_not_emit_audit_event_on_permission_error(
    ) {
        let profile = Profile::default();
        let registration_repo = FakeAccountRegistrationRepo {
            account_id: Uuid::new_v4(),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_system_account(
            profile,
            "system-account".to_string(),
            SystemActor::SystemManager,
            Box::new(&registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_system_account_does_not_emit_audit_event_on_disallowed_actor(
    ) {
        let profile = staff_profile();
        let registration_repo = FakeAccountRegistrationRepo {
            account_id: Uuid::new_v4(),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_system_account(
            profile,
            "system-account".to_string(),
            SystemActor::CustomRole("not-allowed".to_string()),
            Box::new(&registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
