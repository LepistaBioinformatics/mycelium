use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            guest_role::GuestRole,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{GuestRoleUpdating, ResourceAuditLogRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(name = "remove_role_child", skip_all)]
pub async fn remove_role_child(
    profile: Profile,
    guest_role_id: Uuid,
    child_id: Uuid,
    guest_role_updating_repo: Box<&dyn GuestRoleUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
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

    let response = guest_role_updating_repo
        .remove_role_child(guest_role_id, child_id, profile.acc_id)
        .await?;

    if let UpdatingResponseKind::Updated(_) = response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::GuestRole,
            guest_role_id,
            None,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "remove_role_child",
                "childId": child_id,
            }),
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

    fn role(id: Uuid) -> GuestRole {
        GuestRole::new(
            Some(id),
            "Test Role".to_string(),
            Some("desc".to_string()),
            crate::domain::dtos::guest_role::Permission::Read,
            None,
            false,
        )
    }

    struct StubGuestRoleUpdating {
        response: UpdatingResponseKind<Option<GuestRole>>,
    }

    #[async_trait]
    impl GuestRoleUpdating for StubGuestRoleUpdating {
        async fn update(
            &self,
            _: GuestRole,
        ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
            unimplemented!()
        }

        async fn insert_role_child(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
        ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors>
        {
            unimplemented!()
        }

        async fn remove_role_child(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
        ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors>
        {
            Ok(self.response.to_owned())
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    #[tokio::test]
    async fn emits_audit_event_when_child_role_is_removed() {
        let guest_role_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(Some(role(guest_role_id))),
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::GuestRole
                    && event.resource_id == guest_role_id
                    && event.event == ResourceAuditEventKind::Updated
                    && event.tenant_id.is_none()
            })
            .returning(|_| Ok(()));

        let result = remove_role_child(
            staff_profile(),
            guest_role_id,
            child_id,
            Box::new(&updating as &dyn GuestRoleUpdating),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn does_not_emit_audit_event_when_permission_check_fails() {
        let guest_role_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(Some(role(guest_role_id))),
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo.expect_create().times(0);

        let result = remove_role_child(
            Profile::default(),
            guest_role_id,
            child_id,
            Box::new(&updating as &dyn GuestRoleUpdating),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_err());
    }
}
