use crate::{
    domain::{
        dtos::{
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            tenant::TenantMetaKey,
            written_by::WrittenBy,
        },
        entities::{ResourceAuditLogRegistration, TenantDeletion},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, tenant_deletion_repo, audit_repo)
)]
pub async fn delete_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    tenant_deletion_repo: Box<&dyn TenantDeletion>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    let response = tenant_deletion_repo
        .delete_tenant_meta(tenant_id, key.to_owned())
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Emit the audit event
    // ? -----------------------------------------------------------------------

    if let DeletionResponseKind::Deleted = &response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::TenantMeta,
            tenant_id,
            Some(tenant_id),
            ResourceAuditEventKind::Deleted,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "delete_tenant_meta",
                "meta_key": key.to_string(),
            }),
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

    struct MockTenantDeletionRepo {
        should_not_delete: bool,
    }

    #[async_trait]
    impl TenantDeletion for MockTenantDeletionRepo {
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
            unimplemented!()
        }

        async fn delete_tenant_meta(
            &self,
            tenant_id: Uuid,
            _: TenantMetaKey,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            if self.should_not_delete {
                return Ok(DeletionResponseKind::NotDeleted(
                    tenant_id,
                    "not found".to_string(),
                ));
            }

            Ok(DeletionResponseKind::Deleted)
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
    async fn delete_tenant_meta_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let deletion_repo = MockTenantDeletionRepo {
            should_not_delete: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::TenantMeta
                    && event.resource_id == tenant_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Deleted
            })
            .returning(|_| Ok(()));

        let result = delete_tenant_meta(
            profile,
            tenant_id,
            TenantMetaKey::FederalRevenueRegister,
            Box::new(&deletion_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_tenant_meta_does_not_emit_audit_event_on_permission_error()
    {
        let tenant_id = Uuid::new_v4();
        let profile = Profile::default();
        let deletion_repo = MockTenantDeletionRepo {
            should_not_delete: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_tenant_meta(
            profile,
            tenant_id,
            TenantMetaKey::FederalRevenueRegister,
            Box::new(&deletion_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_tenant_meta_does_not_emit_audit_event_when_not_deleted() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let deletion_repo = MockTenantDeletionRepo {
            should_not_delete: true,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = delete_tenant_meta(
            profile,
            tenant_id,
            TenantMetaKey::FederalRevenueRegister,
            Box::new(&deletion_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
