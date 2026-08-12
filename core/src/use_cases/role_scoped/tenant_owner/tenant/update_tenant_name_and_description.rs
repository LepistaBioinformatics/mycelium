use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        tenant::Tenant,
        written_by::WrittenBy,
    },
    entities::{ResourceAuditLogRegistration, TenantFetching, TenantUpdating},
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tenant_name_and_description",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tenant_name_and_description(
    profile: Profile,
    tenant_id: Uuid,
    tenant_name: Option<String>,
    tenant_description: Option<String>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    tenant_updating_repo: Box<&dyn TenantUpdating>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    let mut tenant = match tenant_fetching_repo
        .get_tenant_owned_by_me(tenant_id, profile.get_owners_ids())
        .await?
    {
        FetchResponseKind::Found(tenant) => tenant,
        FetchResponseKind::NotFound(_) => {
            return fetching_err("Tenant not found")
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update tenant
    // ? -----------------------------------------------------------------------

    if let Some(name) = tenant_name {
        tenant.name = name;
    }

    if let Some(description) = tenant_description {
        tenant.description = Some(description);
    }

    let result = tenant_updating_repo
        .update_name_and_description(tenant_id, tenant)
        .await;

    let Ok(UpdatingResponseKind::Updated(_)) = result else {
        return result;
    };

    emit_resource_audit_event(
        audit_repo,
        ResourceAuditResourceType::Tenant,
        tenant_id,
        Some(tenant_id),
        ResourceAuditEventKind::Updated,
        WrittenBy::new_from_account(profile.acc_id),
        serde_json::json!({ "action": "update_tenant_name_and_description" }),
    )
    .await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::tenant::TenantMetaKey;
    use crate::domain::entities::{
        MockResourceAuditLogRegistration, TenantOwnerConnection,
    };

    use async_trait::async_trait;
    use mycelium_base::entities::{CreateResponseKind, FetchManyResponseKind};

    struct MockTenantFetching {
        owner_id: Uuid,
        tenant: Tenant,
    }

    #[async_trait]
    impl TenantFetching for MockTenantFetching {
        async fn get_tenant_owned_by_me(
            &self,
            _: Uuid,
            owners_ids: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            if owners_ids.contains(&self.owner_id) {
                return Ok(FetchResponseKind::Found(self.tenant.clone()));
            }

            Ok(FetchResponseKind::NotFound(None))
        }

        async fn get_tenant_public_by_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_tenants_by_manager_account(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn filter_tenants_as_manager(
            &self,
            _: Option<String>,
            _: Option<Uuid>,
            _: Option<(TenantMetaKey, String)>,
            _: Option<(String, String)>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
            unimplemented!()
        }
    }

    struct MockTenantUpdating {
        response: UpdatingResponseKind<Tenant>,
    }

    #[async_trait]
    impl TenantUpdating for MockTenantUpdating {
        async fn update_name_and_description(
            &self,
            _: Uuid,
            _: Tenant,
        ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
            Ok(self.response.clone())
        }

        async fn update_tenant_status(
            &self,
            _: Uuid,
            _: crate::domain::dtos::tenant::TenantStatus,
        ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
            unimplemented!()
        }

        async fn register_owner(
            &self,
            _: Uuid,
            _: Uuid,
            _: String,
        ) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors>
        {
            unimplemented!()
        }

        async fn update_tenant_meta(
            &self,
            _: Uuid,
            _: TenantMetaKey,
            _: String,
        ) -> Result<
            UpdatingResponseKind<crate::domain::dtos::tenant::TenantMeta>,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    fn sample_tenant(tenant_id: Uuid) -> Tenant {
        let mut tenant = Tenant::new_with_owners(
            "tenant".to_string(),
            None,
            mycelium_base::dtos::Children::Records(vec![]),
        );
        tenant.id = Some(tenant_id);
        tenant
    }

    fn owner_profile(owner_id: Uuid) -> Profile {
        let mut profile = Profile::default();
        profile.owners = vec![crate::domain::dtos::profile::Owner {
            id: owner_id,
            email: "owner@example.com".to_string(),
            first_name: None,
            last_name: None,
            username: None,
            is_principal: true,
        }];
        profile
    }

    #[tokio::test]
    async fn update_tenant_name_and_description_emits_audit_event_on_success() {
        let tenant_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let profile = owner_profile(owner_id);

        let tenant_fetching = MockTenantFetching {
            owner_id,
            tenant: sample_tenant(tenant_id),
        };
        let tenant_updating = MockTenantUpdating {
            response: UpdatingResponseKind::Updated(sample_tenant(tenant_id)),
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

        let result = update_tenant_name_and_description(
            profile,
            tenant_id,
            Some("new name".to_string()),
            None,
            Box::new(&tenant_fetching),
            Box::new(&tenant_updating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_tenant_name_and_description_does_not_emit_audit_event_on_permission_error(
    ) {
        let tenant_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        // Profile has no owners matching the tenant, so the fetching mock
        // reports `NotFound`, mirroring the real repo's "owned by me" filter.
        let profile = Profile::default();

        let tenant_fetching = MockTenantFetching {
            owner_id,
            tenant: sample_tenant(tenant_id),
        };
        let tenant_updating = MockTenantUpdating {
            response: UpdatingResponseKind::Updated(sample_tenant(tenant_id)),
        };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = update_tenant_name_and_description(
            profile,
            tenant_id,
            Some("new name".to_string()),
            None,
            Box::new(&tenant_fetching),
            Box::new(&tenant_updating),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
