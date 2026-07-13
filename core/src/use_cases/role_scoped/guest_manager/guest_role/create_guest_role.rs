use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            guest_role::{GuestRole, Permission},
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{GuestRoleRegistration, ResourceAuditLogRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};

/// Create a new guest role
///
/// This function should be called only by manager users. Roles should be
/// created after the application is registered by staff users why roles links
/// guest, permissions, and applications.
///
/// As example, a group of users need to has view only permissions to resources
/// of a single application. Thus, the role should include only the `View`
/// permission (level zero) for the `Movie` application. Thus, the role name
/// should be: "Movie Viewers".
#[tracing::instrument(name = "create_guest_role", skip_all)]
pub async fn create_guest_role(
    profile: Profile,
    name: String,
    description: String,
    permission: Option<Permission>,
    system: bool,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    let response = guest_role_registration_repo
        .get_or_create(GuestRole::new(
            None,
            name,
            Some(description),
            permission.unwrap_or(Permission::Read),
            None,
            system,
        ))
        .await?;

    // ? ----------------------------------------------------------------------
    // ? Emit audit event when a new role was actually created
    // ? ----------------------------------------------------------------------

    let created_role_id = match &response {
        GetOrCreateResponseKind::Created(role) => role.id,
        GetOrCreateResponseKind::NotCreated(_, _) => None,
    };

    if let Some(role_id) = created_role_id {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::GuestRole,
            role_id,
            None,
            ResourceAuditEventKind::Created,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "create_guest_role" }),
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
    use uuid::Uuid;

    struct StubGuestRoleRegistration {
        role: GuestRole,
    }

    #[async_trait]
    impl GuestRoleRegistration for StubGuestRoleRegistration {
        async fn get_or_create(
            &self,
            _: GuestRole,
        ) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
            Ok(GetOrCreateResponseKind::Created(self.role.clone()))
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    #[tokio::test]
    async fn emits_audit_event_when_role_is_created() {
        let role = GuestRole::new(
            Some(Uuid::new_v4()),
            "Test Role".to_string(),
            Some("desc".to_string()),
            Permission::Read,
            None,
            false,
        );

        let registration = StubGuestRoleRegistration { role };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo
            .expect_create()
            .times(1)
            .withf(|event| {
                event.resource_type == ResourceAuditResourceType::GuestRole
                    && event.event == ResourceAuditEventKind::Created
                    && event.tenant_id.is_none()
            })
            .returning(|_| Ok(()));

        let result = create_guest_role(
            staff_profile(),
            "Test Role".to_string(),
            "desc".to_string(),
            None,
            false,
            Box::new(&registration as &dyn GuestRoleRegistration),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn does_not_emit_audit_event_when_permission_check_fails() {
        let role = GuestRole::new(
            Some(Uuid::new_v4()),
            "Test Role".to_string(),
            Some("desc".to_string()),
            Permission::Read,
            None,
            false,
        );

        let registration = StubGuestRoleRegistration { role };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo.expect_create().times(0);

        let result = create_guest_role(
            Profile::default(),
            "Test Role".to_string(),
            "desc".to_string(),
            None,
            false,
            Box::new(&registration as &dyn GuestRoleRegistration),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_err());
    }
}
