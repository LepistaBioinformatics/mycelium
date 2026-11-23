use crate::{
    domain::{
        dtos::{
            issuable_notification_service::IssuableNotificationService,
            notification_issuance_options::NotificationIssuanceOptions,
            profile::Profile,
            service_notification_config::{
                RecipientPolicy, ServiceNotificationConfig,
            },
        },
        entities::ServiceRead,
    },
    models::ServiceNotificationsConfig,
    use_cases::support::check_notification_issuer,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// List the services the caller can issue notification connection strings
/// for, with the tenants each string can be bound to and the instance TTL
/// cap.
///
/// Tenants are checked with the same rule issuance applies
/// (`check_notification_issuer`), so every option returned here is one
/// `create_notification_connection_string` accepts.
#[tracing::instrument(
    name = "list_notification_issuance_options",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_notification_issuance_options(
    profile: Profile,
    notifications_settings: ServiceNotificationsConfig,
    service_read_repo: Box<&dyn ServiceRead>,
) -> Result<NotificationIssuanceOptions, MappedErrors> {
    let candidate_tenants = licensed_tenants(&profile);

    let mut services: Vec<IssuableNotificationService> =
        notification_configs(service_read_repo)
            .await?
            .iter()
            .filter_map(|config| {
                issuable_service(&profile, &candidate_tenants, config)
            })
            .collect();

    services.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    Ok(NotificationIssuanceOptions {
        max_connection_string_ttl_in_days: notifications_settings
            .max_connection_string_ttl_in_days,
        services,
    })
}

async fn notification_configs(
    service_read_repo: Box<&dyn ServiceRead>,
) -> Result<Vec<ServiceNotificationConfig>, MappedErrors> {
    let services =
        match service_read_repo.list_services(None, None, None).await? {
            FetchManyResponseKind::Found(services) => services,
            FetchManyResponseKind::FoundPaginated { records, .. } => records,
            FetchManyResponseKind::NotFound => vec![],
        };

    Ok(services
        .into_iter()
        .filter_map(|service| service.notifications)
        .collect())
}

fn licensed_tenants(profile: &Profile) -> Vec<Uuid> {
    let mut tenants: Vec<Uuid> = profile
        .licensed_resources
        .as_ref()
        .map(|resources| {
            resources
                .to_licenses_vector()
                .iter()
                .map(|resource| resource.tenant_id)
                .collect()
        })
        .unwrap_or_default();

    tenants.sort();
    tenants.dedup();
    tenants
}

fn issuable_service(
    profile: &Profile,
    candidate_tenants: &[Uuid],
    config: &ServiceNotificationConfig,
) -> Option<IssuableNotificationService> {
    let tenant_ids: Vec<Uuid> = candidate_tenants
        .iter()
        .filter(|tenant_id| {
            check_notification_issuer(profile, Some(**tenant_id), config)
                .is_ok()
        })
        .copied()
        .collect();

    let allows_no_tenant = config.recipient_policy
        != RecipientPolicy::TenantMembers
        && check_notification_issuer(profile, None, config).is_ok();

    if tenant_ids.is_empty() && !allows_no_tenant {
        return None;
    }

    Some(IssuableNotificationService {
        sender_tag: config.sender_tag.to_owned(),
        display_name: config.display_name.to_owned(),
        recipient_policy: config.recipient_policy,
        tenant_ids,
        allows_no_tenant,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            guest_role::Permission,
            profile::{LicensedResource, LicensedResources},
            service::Service,
        },
        entities::MockServiceRead,
    };

    const ISSUER_ROLE: &str = "service-01-admin";

    fn settings() -> ServiceNotificationsConfig {
        serde_json::from_value(serde_json::json!({
            "maxConnectionStringTtlInDays": 30,
        }))
        .unwrap()
    }

    fn service(
        sender_tag: &str,
        recipient_policy: &str,
        issuer_tenants: Vec<Uuid>,
    ) -> Service {
        serde_json::from_value(serde_json::json!({
            "name": sender_tag,
            "host": "localhost:8080",
            "protocol": "http",
            "routes": [],
            "healthCheckPath": "/health",
            "notifications": {
                "senderTag": sender_tag,
                "displayName": format!("Display {sender_tag}"),
                "issuerRoles": [ISSUER_ROLE],
                "issuerTenants": issuer_tenants,
                "recipientPolicy": recipient_policy,
                "templates": ["welcome"],
            },
        }))
        .unwrap()
    }

    fn service_without_notifications() -> Service {
        serde_json::from_value(serde_json::json!({
            "name": "service-02",
            "host": "localhost:8081",
            "protocol": "http",
            "routes": [],
            "healthCheckPath": "/health",
        }))
        .unwrap()
    }

    fn profile(role: &str, tenant_ids: &[Uuid], active: bool) -> Profile {
        let resources = tenant_ids
            .iter()
            .map(|tenant_id| LicensedResource {
                acc_id: Uuid::new_v4(),
                tenant_id: *tenant_id,
                role_id: Uuid::new_v4(),
                acc_name: "Account".to_string(),
                sys_acc: false,
                role: role.to_string(),
                perm: Permission::Write,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            })
            .collect();

        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            active,
            true,
            false,
            false,
            None,
            Some(LicensedResources::Records(resources)),
            None,
        )
    }

    fn repo(services: Vec<Service>) -> MockServiceRead {
        let mut repo = MockServiceRead::new();
        repo.expect_list_services().returning(move |_, _, _| {
            Ok(FetchManyResponseKind::Found(services.clone()))
        });
        repo
    }

    async fn options(
        profile: Profile,
        services: Vec<Service>,
    ) -> NotificationIssuanceOptions {
        let repo = repo(services);
        list_notification_issuance_options(profile, settings(), Box::new(&repo))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn lists_the_issuer_tenants_and_the_ttl_cap() {
        let tenant_id = Uuid::new_v4();
        let result = options(
            profile(ISSUER_ROLE, &[tenant_id], true),
            vec![
                service("service-01", "owner", vec![]),
                service_without_notifications(),
            ],
        )
        .await;

        assert_eq!(result.max_connection_string_ttl_in_days, 30);
        assert_eq!(result.services.len(), 1);
        assert_eq!(result.services[0].sender_tag, "service-01");
        assert_eq!(result.services[0].tenant_ids, vec![tenant_id]);
        assert!(result.services[0].allows_no_tenant);
    }

    #[tokio::test]
    async fn tenant_members_policy_requires_a_tenant() {
        let tenant_id = Uuid::new_v4();
        let result = options(
            profile(ISSUER_ROLE, &[tenant_id], true),
            vec![service("service-01", "tenantMembers", vec![])],
        )
        .await;

        assert!(!result.services[0].allows_no_tenant);
        assert_eq!(result.services[0].tenant_ids, vec![tenant_id]);
    }

    #[tokio::test]
    async fn issuer_tenants_restrict_the_tenants() {
        let allowed = Uuid::new_v4();
        let other = Uuid::new_v4();
        let result = options(
            profile(ISSUER_ROLE, &[allowed, other], true),
            vec![service("service-01", "owner", vec![allowed])],
        )
        .await;

        assert_eq!(result.services[0].tenant_ids, vec![allowed]);
        assert!(!result.services[0].allows_no_tenant);
    }

    #[tokio::test]
    async fn omits_services_the_caller_cannot_issue_for() {
        let result = options(
            profile("other-role", &[Uuid::new_v4()], true),
            vec![service("service-01", "owner", vec![])],
        )
        .await;

        assert!(result.services.is_empty());
    }

    #[tokio::test]
    async fn inactive_accounts_get_no_services() {
        let result = options(
            profile(ISSUER_ROLE, &[Uuid::new_v4()], false),
            vec![service("service-01", "owner", vec![])],
        )
        .await;

        assert!(result.services.is_empty());
    }

    #[tokio::test]
    async fn services_are_sorted_by_display_name() {
        let tenant_id = Uuid::new_v4();
        let result = options(
            profile(ISSUER_ROLE, &[tenant_id], true),
            vec![
                service("service-02", "owner", vec![]),
                service("service-01", "owner", vec![]),
            ],
        )
        .await;

        let tags: Vec<&str> = result
            .services
            .iter()
            .map(|service| service.sender_tag.as_str())
            .collect();
        assert_eq!(tags, vec!["service-01", "service-02"]);
    }
}
