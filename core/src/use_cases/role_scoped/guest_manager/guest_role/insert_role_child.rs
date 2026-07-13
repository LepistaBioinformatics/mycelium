use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            guest_role::GuestRole,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            written_by::WrittenBy,
        },
        entities::{
            GuestRoleFetching, GuestRoleUpdating, ResourceAuditLogRegistration,
        },
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use futures::future;
use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(name = "insert_role_child", skip_all)]
pub async fn insert_role_child(
    profile: Profile,
    guest_role_id: Uuid,
    child_id: Uuid,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_role_updating_repo: Box<&dyn GuestRoleUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch target and child roles
    //
    // This check is necessary to guarantee that the child guest-role has the
    // same role that the target role.
    //
    // ? -----------------------------------------------------------------------

    if guest_role_id == child_id {
        return use_case_err(
            "The target role and the child role must be different",
        )
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00018)
        .as_error();
    }

    let (target_role, children_role) = future::join(
        guest_role_fetching_repo.get(guest_role_id),
        guest_role_fetching_repo.get(child_id),
    )
    .await;

    let target_role = match target_role? {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to check target role: {}",
                guest_role_id,
            ))
            .as_error();
        }
        FetchResponseKind::Found(role) => role.permission,
    };

    let children_role = match children_role? {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to check child role: {}",
                child_id,
            ))
            .as_error();
        }
        FetchResponseKind::Found(role) => role.permission,
    };

    if target_role.to_i32() < children_role.to_i32() {
        return use_case_err(
            "Only roles with higher permission level can be children of a role",
        )
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00018)
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Persist UserRole
    // ? -----------------------------------------------------------------------

    let response = guest_role_updating_repo
        .insert_role_child(guest_role_id, child_id, profile.acc_id)
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
                "action": "insert_role_child",
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
    use crate::domain::dtos::guest_role::Permission;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;

    fn role(id: Uuid, permission: Permission) -> GuestRole {
        GuestRole::new(
            Some(id),
            "Test Role".to_string(),
            Some("desc".to_string()),
            permission,
            None,
            false,
        )
    }

    struct StubGuestRoleFetching {
        target: GuestRole,
        child: GuestRole,
    }

    #[async_trait]
    impl GuestRoleFetching for StubGuestRoleFetching {
        async fn get(
            &self,
            id: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            let role = match id {
                _ if Some(id) == self.target.id => self.target.to_owned(),
                _ => self.child.to_owned(),
            };

            Ok(FetchResponseKind::Found(role))
        }

        async fn get_parent_by_child_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            unimplemented!()
        }

        async fn list(
            &self,
            _: Option<String>,
            _: Option<String>,
            _: Option<bool>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<GuestRole>,
            MappedErrors,
        > {
            unimplemented!()
        }
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
            Ok(self.response.to_owned())
        }

        async fn remove_role_child(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
        ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors>
        {
            unimplemented!()
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    #[tokio::test]
    async fn emits_audit_event_when_child_role_is_inserted() {
        let guest_role_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let fetching = StubGuestRoleFetching {
            target: role(guest_role_id, Permission::Write),
            child: role(child_id, Permission::Read),
        };
        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(Some(role(
                guest_role_id,
                Permission::Write,
            ))),
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

        let result = insert_role_child(
            staff_profile(),
            guest_role_id,
            child_id,
            Box::new(&fetching as &dyn GuestRoleFetching),
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

        let fetching = StubGuestRoleFetching {
            target: role(guest_role_id, Permission::Write),
            child: role(child_id, Permission::Read),
        };
        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(Some(role(
                guest_role_id,
                Permission::Write,
            ))),
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo.expect_create().times(0);

        let result = insert_role_child(
            Profile::default(),
            guest_role_id,
            child_id,
            Box::new(&fetching as &dyn GuestRoleFetching),
            Box::new(&updating as &dyn GuestRoleUpdating),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn does_not_emit_audit_event_when_target_and_child_are_same() {
        let guest_role_id = Uuid::new_v4();

        let fetching = StubGuestRoleFetching {
            target: role(guest_role_id, Permission::Write),
            child: role(guest_role_id, Permission::Write),
        };
        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(Some(role(
                guest_role_id,
                Permission::Write,
            ))),
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo.expect_create().times(0);

        let result = insert_role_child(
            staff_profile(),
            guest_role_id,
            guest_role_id,
            Box::new(&fetching as &dyn GuestRoleFetching),
            Box::new(&updating as &dyn GuestRoleUpdating),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_err());
    }
}
