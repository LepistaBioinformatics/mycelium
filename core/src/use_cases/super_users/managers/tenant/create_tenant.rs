use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::{Owner, Profile},
        resource_audit_log::{
            ResourceAuditEventKind, ResourceAuditResourceType,
        },
        tenant::Tenant,
        written_by::WrittenBy,
    },
    entities::{
        ResourceAuditLogRegistration, TenantRegistration, UserFetching,
    },
};
use crate::use_cases::shared::audit::emit_resource_audit_event;

use mycelium_base::{
    dtos::Children,
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "create_tenant",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, user_fetching_repo, tenant_registration_repo, audit_repo)
)]
pub async fn create_tenant(
    profile: Profile,
    tenant_name: String,
    tenant_description: Option<String>,
    tenant_owner_id: Uuid,
    user_fetching_repo: Box<&dyn UserFetching>,
    tenant_registration_repo: Box<&dyn TenantRegistration>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Check if the proposed tenant owner exists
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo.get_user_by_id(tenant_owner_id).await? {
        FetchResponseKind::Found(res) => res,
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "User with ID {} not already registered",
                tenant_owner_id
            ))
            .with_code(NativeErrorCodes::MYC00009)
            .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Initialize tenant object
    // ? -----------------------------------------------------------------------

    let tenant = Tenant::new_with_owners(
        tenant_name,
        tenant_description,
        Children::Records(vec![Owner::from_user(user)?]),
    );

    // ? -----------------------------------------------------------------------
    // ? Register tenant
    // ? -----------------------------------------------------------------------

    let result = tenant_registration_repo
        .create(tenant, format!("account-id:{}", profile.acc_id))
        .await;

    let Ok(CreateResponseKind::Created(ref created_tenant)) = result else {
        return result;
    };

    let Some(tenant_id) = created_tenant.id else {
        return result;
    };

    emit_resource_audit_event(
        audit_repo,
        ResourceAuditResourceType::Tenant,
        tenant_id,
        Some(tenant_id),
        ResourceAuditEventKind::Created,
        WrittenBy::new_from_account(profile.acc_id),
        serde_json::json!({ "action": "create_tenant" }),
    )
    .await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::email::Email;
    use crate::domain::entities::MockResourceAuditLogRegistration;

    use async_trait::async_trait;
    use chrono::Local;
    use std::collections::HashMap;

    struct MockUserFetching {
        user_id: Uuid,
    }

    #[async_trait]
    impl UserFetching for MockUserFetching {
        async fn get_user_by_email(
            &self,
            _: Email,
        ) -> Result<
            FetchResponseKind<crate::domain::dtos::user::User, String>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn get_user_by_id(
            &self,
            _: Uuid,
        ) -> Result<
            FetchResponseKind<crate::domain::dtos::user::User, String>,
            MappedErrors,
        > {
            Ok(FetchResponseKind::Found(
                crate::domain::dtos::user::User::new(
                    Some(self.user_id),
                    "owner".to_string(),
                    Email::from_string("owner@example.com".to_string())?,
                    None,
                    None,
                    true,
                    Local::now(),
                    None,
                    None,
                    None,
                ),
            ))
        }

        async fn get_not_redacted_user_by_email(
            &self,
            _: Email,
        ) -> Result<
            FetchResponseKind<crate::domain::dtos::user::User, String>,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    struct MockTenantRegistration {
        tenant_id: Uuid,
    }

    #[async_trait]
    impl TenantRegistration for MockTenantRegistration {
        async fn create(
            &self,
            tenant: Tenant,
            _: String,
        ) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
            let mut created = tenant;
            created.id = Some(self.tenant_id);
            Ok(CreateResponseKind::Created(created))
        }

        async fn register_tenant_meta(
            &self,
            _: Vec<Uuid>,
            _: Uuid,
            _: crate::domain::dtos::tenant::TenantMetaKey,
            _: String,
        ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>
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
    async fn create_tenant_emits_audit_event_on_success() {
        let profile = staff_profile();
        let tenant_owner_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let user_fetching = MockUserFetching {
            user_id: tenant_owner_id,
        };
        let tenant_registration = MockTenantRegistration { tenant_id };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock
            .expect_create()
            .times(1)
            .withf(move |event| {
                event.resource_type == ResourceAuditResourceType::Tenant
                    && event.resource_id == tenant_id
                    && event.tenant_id == Some(tenant_id)
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        let result = create_tenant(
            profile,
            "tenant".to_string(),
            None,
            tenant_owner_id,
            Box::new(&user_fetching),
            Box::new(&tenant_registration),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_tenant_does_not_emit_audit_event_on_permission_error() {
        let profile = Profile::default();
        let tenant_owner_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let user_fetching = MockUserFetching {
            user_id: tenant_owner_id,
        };
        let tenant_registration = MockTenantRegistration { tenant_id };

        let mut audit_mock = MockResourceAuditLogRegistration::new();
        audit_mock.expect_create().times(0);

        let result = create_tenant(
            profile,
            "tenant".to_string(),
            None,
            tenant_owner_id,
            Box::new(&user_fetching),
            Box::new(&tenant_registration),
            Box::new(&audit_mock),
        )
        .await;

        assert!(result.is_err());
    }
}
