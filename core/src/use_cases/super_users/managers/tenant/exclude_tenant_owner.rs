use crate::domain::{
    dtos::{
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{ResourceAuditLogRegistration, TenantDeletion},
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "exclude_tenant_owner",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_deletion_repo, audit_repo))
]
pub async fn exclude_tenant_owner(
    profile: Profile,
    tenant_id: Uuid,
    owner_id: Uuid,
    tenant_deletion_repo: Box<&dyn TenantDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Delete owner
    // ? -----------------------------------------------------------------------

    let result = tenant_deletion_repo
        .delete_owner(tenant_id, Some(owner_id), None)
        .await;

    let Ok(DeletionResponseKind::Deleted) = result else {
        return result;
    };

    emit_resource_audit_event(
        audit_repo,
        ResourceAuditResourceType::Tenant,
        tenant_id,
        Some(tenant_id),
        ResourceAuditEventKind::Updated,
        WrittenBy::new_from_account(profile.acc_id),
        serde_json::json!({ "action": "exclude_tenant_owner", "owner_id": owner_id }),
    )
    .await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;

    struct MockTenantDeletion {
        response: DeletionResponseKind<Uuid>,
    }

    #[async_trait]
    impl TenantDeletion for MockTenantDeletion {
        async fn delete(
            &self,
            _: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
        }

        async fn delete_owner(
            &self,
            _: Uuid,
            _: Option<Uuid>,
            _: Option<crate::domain::dtos::email::Email>,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            Ok(self.response.clone())
        }

        async fn delete_tenant_meta(
            &self,
            _: Uuid,
            _: crate::domain::dtos::tenant::TenantMetaKey,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    #[tokio::test]
    async fn exclude_tenant_owner_emits_audit_event_on_success() {
        let profile = staff_profile();
        let tenant_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let tenant_deletion = MockTenantDeletion {
            response: DeletionResponseKind::Deleted,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Tenant
                    && event.resource_id == tenant_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Updated
            })
            .returning(|_| Ok(()));

        let result = exclude_tenant_owner(
            profile,
            tenant_id,
            owner_id,
            Box::new(&tenant_deletion),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn exclude_tenant_owner_does_not_emit_audit_event_on_permission_error(
    ) {
        let profile = Profile::default();
        let tenant_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let tenant_deletion = MockTenantDeletion {
            response: DeletionResponseKind::Deleted,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = exclude_tenant_owner(
            profile,
            tenant_id,
            owner_id,
            Box::new(&tenant_deletion),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
