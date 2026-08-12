use crate::domain::{
    dtos::{
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::{
        ResourceAuditLogRegistration, TenantOwnerConnection, TenantUpdating,
    },
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "include_tenant_owner",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_updating_repo, audit_repo))]
pub async fn include_tenant_owner(
    profile: Profile,
    tenant_id: Uuid,
    owner_id: Uuid,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Delete owner
    // ? -----------------------------------------------------------------------

    let result = tenant_updating_repo
        .register_owner(
            tenant_id,
            owner_id,
            format!("account-id:{}", profile.acc_id),
        )
        .await;

    let Ok(CreateResponseKind::Created(_)) = result else {
        return result;
    };

    emit_resource_audit_event(
        audit_repo,
        ResourceAuditResourceType::Tenant,
        tenant_id,
        Some(tenant_id),
        ResourceAuditEventKind::Updated,
        WrittenBy::new_from_account(profile.acc_id),
        serde_json::json!({ "action": "include_tenant_owner", "owner_id": owner_id }),
    )
    .await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;
    use chrono::Local;

    struct MockTenantUpdating {
        response: CreateResponseKind<TenantOwnerConnection>,
    }

    #[async_trait]
    impl TenantUpdating for MockTenantUpdating {
        async fn update_name_and_description(
            &self,
            _: Uuid,
            _: crate::domain::dtos::tenant::Tenant,
        ) -> Result<
            mycelium_base::entities::UpdatingResponseKind<
                crate::domain::dtos::tenant::Tenant,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn update_tenant_status(
            &self,
            _: Uuid,
            _: crate::domain::dtos::tenant::TenantStatus,
        ) -> Result<
            mycelium_base::entities::UpdatingResponseKind<
                crate::domain::dtos::tenant::Tenant,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn register_owner(
            &self,
            _: Uuid,
            _: Uuid,
            _: String,
        ) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors>
        {
            Ok(self.response.clone())
        }

        async fn update_tenant_meta(
            &self,
            _: Uuid,
            _: crate::domain::dtos::tenant::TenantMetaKey,
            _: String,
        ) -> Result<
            mycelium_base::entities::UpdatingResponseKind<
                crate::domain::dtos::tenant::TenantMeta,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    fn staff_profile() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    fn sample_connection(
        tenant_id: Uuid,
        owner_id: Uuid,
    ) -> TenantOwnerConnection {
        TenantOwnerConnection {
            tenant_id,
            owner_id,
            guest_by: "account-id:test".to_string(),
            created: Local::now(),
            updated: None,
        }
    }

    #[tokio::test]
    async fn include_tenant_owner_emits_audit_event_on_success() {
        let profile = staff_profile();
        let tenant_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let tenant_updating = MockTenantUpdating {
            response: CreateResponseKind::Created(sample_connection(
                tenant_id, owner_id,
            )),
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

        let result = include_tenant_owner(
            profile,
            tenant_id,
            owner_id,
            Box::new(&tenant_updating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn include_tenant_owner_does_not_emit_audit_event_on_permission_error(
    ) {
        let profile = Profile::default();
        let tenant_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let tenant_updating = MockTenantUpdating {
            response: CreateResponseKind::Created(sample_connection(
                tenant_id, owner_id,
            )),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = include_tenant_owner(
            profile,
            tenant_id,
            owner_id,
            Box::new(&tenant_updating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
