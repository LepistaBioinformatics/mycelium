use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account,
        guest_role::{GuestRole, Permission},
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{
        AccountRegistration, GuestRoleRegistration,
        ResourceAuditLogRegistration,
    },
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use futures::future;
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
use tracing::Instrument;
use uuid::Uuid;

/// Create a role associated account
///
/// The role associated account should be tenant-scoped and use the
/// AccountType::RoleAssociated actor associated option.
///
#[tracing::instrument(
    name = "create_role_associated_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
        correspondence_id = tracing::field::Empty
    ),
    skip(
        profile,
        account_registration_repo,
        guest_role_registration_repo,
        audit_repo
    )
)]
pub async fn create_role_associated_account(
    profile: Profile,
    tenant_id: Uuid,
    account_name: String,
    role_name: String,
    role_description: String,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    let span = tracing::Span::current();
    span.record("correspondence_id", Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a role associated account");

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let is_owner = profile.with_tenant_ownership_or_error(tenant_id).is_ok();

    let has_access = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()
        .is_ok();

    if ![is_owner, has_access].iter().any(|&x| x) {
        return use_case_err(
            "Insufficient privileges to create a role associated account",
        )
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Get or create the role
    //
    // The role should be fetched from the database.
    // ? -----------------------------------------------------------------------

    let (id, role_slug, description, children, is_system) = (
        None,
        slugify!(role_name.as_str()),
        Some(role_description),
        None,
        false,
    );

    // ? -----------------------------------------------------------------------
    // ? Get or create the read/write roles
    //
    // The roles are fetched from the database if exists, otherwise they are
    // created.
    // ? -----------------------------------------------------------------------

    let (read_role_result, write_role_result) = future::join(
        guest_role_registration_repo
            .get_or_create(GuestRole::new(
                id,
                role_slug.to_owned(),
                description.to_owned(),
                Permission::Read,
                children.to_owned(),
                is_system.to_owned(),
            ))
            .instrument(span.clone()),
        guest_role_registration_repo
            .get_or_create(GuestRole::new(
                id,
                role_slug.to_owned(),
                description,
                Permission::Write,
                children,
                is_system,
            ))
            .instrument(span.clone()),
    )
    .await;

    let read_role_id = match read_role_result? {
        GetOrCreateResponseKind::Created(role) => role,
        GetOrCreateResponseKind::NotCreated(role, _) => role,
    }
    .id
    .map_or_else(
        || {
            use_case_err(format!("Role ID is not set: {}", tenant_id))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        },
        #[allow(clippy::needless_collect)]
        |id| Ok(id),
    )?;

    let write_role_id = match write_role_result? {
        GetOrCreateResponseKind::Created(role) => role,
        GetOrCreateResponseKind::NotCreated(role, _) => role,
    }
    .id
    .map_or_else(
        || {
            use_case_err(format!("Role ID is not set: {}", tenant_id))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        },
        #[allow(clippy::needless_collect)]
        |id| Ok(id),
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let account_name_base_slug = slugify!(format!(
        "tid/{}/role/{}/role-associated-account",
        tenant_id, role_slug
    )
    .as_str());

    let performed_by = WrittenBy::new_from_account(profile.acc_id);

    let mut unchecked_account = Account::new_role_related_account(
        account_name,
        tenant_id,
        read_role_id,
        write_role_id,
        role_slug,
        false,
        Some(performed_by.to_owned()),
    );

    unchecked_account.slug = account_name_base_slug;
    unchecked_account.is_checked = true;

    let response = account_registration_repo
        .get_or_create_role_related_account(unchecked_account)
        .instrument(span)
        .await?;

    if let GetOrCreateResponseKind::Created(account) = &response {
        let account_id = account.id.ok_or_else(|| {
            use_case_err("Account ID not found".to_string()).with_exp_true()
        })?;

        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Account,
            account_id,
            Some(tenant_id),
            ResourceAuditEventKind::Created,
            performed_by,
            serde_json::json!({ "action": "create_role_associated_account" }),
        )
        .await;
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::profile::{TenantOwnership, TenantsOwnership},
        entities::MockResourceAuditLogRegistration,
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::CreateResponseKind;
    use std::collections::HashMap;

    struct FakeGuestRoleRegistrationRepo;

    #[async_trait]
    impl GuestRoleRegistration for FakeGuestRoleRegistrationRepo {
        async fn get_or_create(
            &self,
            mut guest_role: GuestRole,
        ) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
            guest_role.id = Some(Uuid::new_v4());
            Ok(GetOrCreateResponseKind::Created(guest_role))
        }
    }

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
            mut account: Account,
        ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
            account.id = Some(self.account_id);
            Ok(GetOrCreateResponseKind::Created(account))
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
        ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>
        {
            unimplemented!()
        }
    }

    fn profile_owning_tenant(tenant_id: Uuid) -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            Some(TenantsOwnership::Records(vec![TenantOwnership {
                id: tenant_id,
                name: "Tenant Name".to_string(),
                since: Local::now(),
            }])),
        )
    }

    #[tokio::test]
    async fn create_role_associated_account_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let guest_role_registration_repo = FakeGuestRoleRegistrationRepo;
        let account_registration_repo =
            FakeAccountRegistrationRepo { account_id };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Account
                    && event.resource_id == account_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        let result = create_role_associated_account(
            profile,
            tenant_id,
            "Role Associated Account".to_string(),
            "role-name".to_string(),
            "role description".to_string(),
            Box::new(&guest_role_registration_repo),
            Box::new(&account_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_role_associated_account_does_not_emit_audit_event_on_permission_error(
    ) {
        let tenant_id = Uuid::new_v4();
        let profile = Profile::default();
        let guest_role_registration_repo = FakeGuestRoleRegistrationRepo;
        let account_registration_repo = FakeAccountRegistrationRepo {
            account_id: Uuid::new_v4(),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_role_associated_account(
            profile,
            tenant_id,
            "Role Associated Account".to_string(),
            "role-name".to_string(),
            "role description".to_string(),
            Box::new(&guest_role_registration_repo),
            Box::new(&account_registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
