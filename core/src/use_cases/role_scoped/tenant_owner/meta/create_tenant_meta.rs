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
        entities::{ResourceAuditLogRegistration, TenantRegistration},
    },
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tracing::instrument(
    name = "create_tenant_meta",
    fields(profile_id = %profile.acc_id),
    skip(key, value, tenant_registration_repo, audit_repo)
)]
pub async fn create_tenant_meta(
    profile: Profile,
    tenant_id: Uuid,
    key: TenantMetaKey,
    value: String,
    tenant_registration_repo: Box<&dyn TenantRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    let response = tenant_registration_repo
        .register_tenant_meta(
            profile.get_owners_ids(),
            tenant_id,
            key.to_owned(),
            value,
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Emit the audit event
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::Created(_) = &response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::TenantMeta,
            tenant_id,
            Some(tenant_id),
            ResourceAuditEventKind::Created,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({
                "action": "create_tenant_meta",
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

    struct MockTenantRegistrationRepo {
        should_not_create: bool,
    }

    #[async_trait]
    impl TenantRegistration for MockTenantRegistrationRepo {
        async fn create(
            &self,
            _: crate::domain::dtos::tenant::Tenant,
            _: String,
        ) -> Result<
            CreateResponseKind<crate::domain::dtos::tenant::Tenant>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn register_tenant_meta(
            &self,
            _: Vec<Uuid>,
            _: Uuid,
            _: TenantMetaKey,
            value: String,
        ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>
        {
            let mut map = HashMap::new();
            map.insert("federal_revenue_register".to_string(), value);

            if self.should_not_create {
                return Ok(CreateResponseKind::NotCreated(
                    map,
                    "already exists".to_string(),
                ));
            }

            Ok(CreateResponseKind::Created(map))
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
    async fn create_tenant_meta_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let registration_repo = MockTenantRegistrationRepo {
            should_not_create: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::TenantMeta
                    && event.resource_id == tenant_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        let result = create_tenant_meta(
            profile,
            tenant_id,
            TenantMetaKey::FederalRevenueRegister,
            "some-value".to_string(),
            Box::new(&registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_tenant_meta_does_not_emit_audit_event_on_permission_error()
    {
        let tenant_id = Uuid::new_v4();
        let profile = Profile::default();
        let registration_repo = MockTenantRegistrationRepo {
            should_not_create: false,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_tenant_meta(
            profile,
            tenant_id,
            TenantMetaKey::FederalRevenueRegister,
            "some-value".to_string(),
            Box::new(&registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_tenant_meta_does_not_emit_audit_event_when_not_created() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_owning_tenant(tenant_id);
        let registration_repo = MockTenantRegistrationRepo {
            should_not_create: true,
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_tenant_meta(
            profile,
            tenant_id,
            TenantMetaKey::FederalRevenueRegister,
            "some-value".to_string(),
            Box::new(&registration_repo),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }
}
