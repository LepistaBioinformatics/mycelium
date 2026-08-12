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
        entities::{
            GuestRoleFetching, GuestRoleUpdating, ResourceAuditLogRegistration,
        },
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function allows only the update of name and description attributes of
/// a single role.
#[tracing::instrument(
    name = "update_guest_role_name_and_description",
    skip_all
)]
pub async fn update_guest_role_name_and_description(
    profile: Profile,
    name: Option<String>,
    description: Option<String>,
    guest_role_id: Uuid,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_role_updating_repo: Box<&dyn GuestRoleUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check the profile permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? ----------------------------------------------------------------------
    // ? Fetch role from data persistence layer
    // ? ----------------------------------------------------------------------

    let mut user_role =
        match guest_role_fetching_repo.get(guest_role_id).await? {
            FetchResponseKind::NotFound(id) => {
                return use_case_err(format!(
                    "Unable to update record: {}",
                    id.unwrap()
                ))
                .as_error();
            }
            FetchResponseKind::Found(role) => role,
        };

    // ? ----------------------------------------------------------------------
    // ? Update value of fetched object
    // ? ----------------------------------------------------------------------

    if name.is_some() {
        user_role.name = name.unwrap();
    };

    if description.is_some() {
        user_role.description = description;
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the updating operation
    // ? ----------------------------------------------------------------------

    let response = guest_role_updating_repo.update(user_role).await?;

    if let UpdatingResponseKind::Updated(ref role) = response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::GuestRole,
            guest_role_id,
            None,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "update_guest_role_name_and_description",
                "name": role.name,
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

    struct StubGuestRoleFetching {
        role: GuestRole,
    }

    #[async_trait]
    impl GuestRoleFetching for StubGuestRoleFetching {
        async fn get(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::Found(self.role.to_owned()))
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
        response: UpdatingResponseKind<GuestRole>,
    }

    #[async_trait]
    impl GuestRoleUpdating for StubGuestRoleUpdating {
        async fn update(
            &self,
            _: GuestRole,
        ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
            Ok(self.response.to_owned())
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
            unimplemented!()
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    #[tokio::test]
    async fn emits_audit_event_when_role_is_updated() {
        let guest_role_id = Uuid::new_v4();
        let updated_role = role(guest_role_id);

        let fetching = StubGuestRoleFetching {
            role: role(guest_role_id),
        };
        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(updated_role),
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

        let result = update_guest_role_name_and_description(
            staff_profile(),
            Some("New Name".to_string()),
            None,
            guest_role_id,
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

        let fetching = StubGuestRoleFetching {
            role: role(guest_role_id),
        };
        let updating = StubGuestRoleUpdating {
            response: UpdatingResponseKind::Updated(role(guest_role_id)),
        };

        let mut audit_repo = MockResourceAuditLogRegistration::new();
        audit_repo.expect_create().times(0);

        let result = update_guest_role_name_and_description(
            Profile::default(),
            Some("New Name".to_string()),
            None,
            guest_role_id,
            Box::new(&fetching as &dyn GuestRoleFetching),
            Box::new(&updating as &dyn GuestRoleUpdating),
            Box::new(&audit_repo),
        )
        .await;

        assert!(result.is_err());
    }
}
