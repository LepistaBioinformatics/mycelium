use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{GuestRoleDeletion, ResourceAuditLogRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// This function deletes a single role. Only manager user could execute such
/// operation.
#[tracing::instrument(name = "delete_guest_role", skip_all)]
pub async fn delete_guest_role(
    profile: Profile,
    guest_role_id: Uuid,
    role_deletion_repo: Box<&dyn GuestRoleDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check user permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? ----------------------------------------------------------------------
    // ? Perform the deletion operation
    // ? ----------------------------------------------------------------------

    let response = role_deletion_repo.delete(guest_role_id).await?;

    if let DeletionResponseKind::Deleted = response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::GuestRole,
            guest_role_id,
            None,
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "delete_guest_role" }),
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

    struct StubGuestRoleDeletion {
        response: DeletionResponseKind<Uuid>,
    }

    #[async_trait]
    impl GuestRoleDeletion for StubGuestRoleDeletion {
        async fn delete(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            Ok(self.response.to_owned())
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    #[tokio::test]
    async fn emits_audit_event_when_role_is_deleted() {
        let guest_role_id = Uuid::new_v4();

        let deletion = StubGuestRoleDeletion {
            response: DeletionResponseKind::Deleted,
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::GuestRole
                    && event.resource_id == guest_role_id
                    && event.event == ResourceAuditEventKind::Deleted
                    && event.tenant_id.is_none()
            })
            .returning(|_| Ok(()));

        let result = delete_guest_role(
            staff_profile(),
            guest_role_id,
            Box::new(&deletion as &dyn GuestRoleDeletion),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn does_not_emit_audit_event_when_permission_check_fails() {
        let guest_role_id = Uuid::new_v4();

        let deletion = StubGuestRoleDeletion {
            response: DeletionResponseKind::Deleted,
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo.expect_create().times(0);

        let result = delete_guest_role(
            Profile::default(),
            guest_role_id,
            Box::new(&deletion as &dyn GuestRoleDeletion),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_err());
    }
}
