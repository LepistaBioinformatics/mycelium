use super::register_and_notify_connection_string::register_and_notify_connection_string;
use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            service_notification_config::{
                RecipientPolicy, ServiceNotificationConfig,
            },
            token::{
                NotificationScope, UserAccountConnectionString,
                UserAccountScope,
            },
        },
        entities::{
            LocalMessageWrite, ServiceRead, TenantFetching, TokenRegistration,
        },
    },
    models::{AccountLifeCycle, ServiceNotificationsConfig},
    use_cases::support::{
        check_notification_issuer, fetch_service_notification_config,
    },
};

use chrono::{Duration, Local};
use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

const SECONDS_PER_DAY: i64 = 86_400;

/// Create a notification connection string
///
/// The string lets a downstream service send emails through the gateway on
/// behalf of the issuing account. It carries the `notify:email` scope and the
/// service sender tag, so it is rejected on every other gateway surface.
///
/// `expiration` is in seconds and must be positive and not above the
/// instance `maxConnectionStringTtlInDays`.
#[tracing::instrument(
    name = "create_notification_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn create_notification_connection_string(
    profile: Profile,
    name: String,
    expiration: i64,
    tenant_id: Option<Uuid>,
    sender_tag: String,
    notifications_settings: ServiceNotificationsConfig,
    life_cycle_settings: AccountLifeCycle,
    service_read_repo: Box<&dyn ServiceRead>,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the request against the service notification settings
    // ? -----------------------------------------------------------------------

    let owner_email = fetch_principal_owner_email(&profile)?;

    check_expiration(expiration, &notifications_settings)?;

    let service_settings =
        fetch_service_notification_config(&sender_tag, service_read_repo)
            .await?;

    check_tenant_requirement(tenant_id, &service_settings)?;

    check_notification_issuer(&profile, tenant_id, &service_settings)?;

    // ? -----------------------------------------------------------------------
    // ? Build the notification scoped token
    // ? -----------------------------------------------------------------------

    let expires_at = Local::now() + Duration::seconds(expiration);

    let mut notification_scope = UserAccountScope::new_notification(
        profile.acc_id,
        expires_at,
        tenant_id,
        vec![NotificationScope::NotifyEmail],
        sender_tag,
        life_cycle_settings.to_owned(),
    )
    .await?;

    let connection_string = UserAccountConnectionString::new_signed_token(
        &mut notification_scope,
        profile.acc_id,
        owner_email.to_owned(),
        life_cycle_settings.to_owned(),
        Some(name),
    )
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Register the token and notify the owner
    // ? -----------------------------------------------------------------------

    register_and_notify_connection_string(
        connection_string,
        expires_at,
        expiration,
        tenant_id,
        owner_email,
        life_cycle_settings,
        token_registration_repo,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
}

fn fetch_principal_owner_email(
    profile: &Profile,
) -> Result<Email, MappedErrors> {
    let Some(owner) = profile.owners.iter().find(|owner| owner.is_principal)
    else {
        return use_case_err(
            "Action restricted to the principal user from the current profile",
        )
        .with_code(NativeErrorCodes::MYC00013)
        .as_error();
    };

    Email::from_string(owner.email.to_owned())
}

fn check_expiration(
    expiration: i64,
    notifications_settings: &ServiceNotificationsConfig,
) -> Result<(), MappedErrors> {
    let max_expiration =
        i64::from(notifications_settings.max_connection_string_ttl_in_days)
            * SECONDS_PER_DAY;

    if expiration > 0 && expiration <= max_expiration {
        return Ok(());
    }

    use_case_err(format!(
        "Expiration must be positive and at most {max_expiration} seconds"
    ))
    .with_code(NativeErrorCodes::MYC00035)
    .with_exp_true()
    .as_error()
}

fn check_tenant_requirement(
    tenant_id: Option<Uuid>,
    service_settings: &ServiceNotificationConfig,
) -> Result<(), MappedErrors> {
    if tenant_id.is_some()
        || service_settings.recipient_policy != RecipientPolicy::TenantMembers
    {
        return Ok(());
    }

    use_case_err("This service requires a tenant to issue notifications")
        .with_code(NativeErrorCodes::MYC00035)
        .with_exp_true()
        .as_error()
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            dtos::{
                guest_role::Permission,
                profile::{LicensedResource, LicensedResources, Owner},
                service::Service,
                token::{MultiTypeMeta, Token},
            },
            entities::{
                MockLocalMessageWrite, MockServiceRead, MockTenantFetching,
                MockTokenRegistration,
            },
        },
        models::{HmacSecretEntry, HmacSecretSet},
    };
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::CreateResponseKind;
    use mycelium_base::entities::FetchManyResponseKind;

    const SENDER_TAG: &str = "service-01";
    const ISSUER_ROLE: &str = "service-01-admin";
    const OWNER_EMAIL: &str = "owner@test.com";
    const ONE_HOUR: i64 = 3600;

    // ? -----------------------------------------------------------------------
    // ? Fixtures
    // ? -----------------------------------------------------------------------

    fn life_cycle() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_url: None,
            domain_name: SecretResolver::Value("Test Domain".to_string()),
            locale: None,
            token_expiration: SecretResolver::Value(30),
            noreply_name: None,
            noreply_email: SecretResolver::Value("noreply@test.com".into()),
            support_name: None,
            support_email: SecretResolver::Value("support@test.com".into()),
            token_secret: SecretResolver::Value("test-secret".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("test-hmac".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    fn notifications_settings() -> ServiceNotificationsConfig {
        serde_json::from_value(serde_json::json!({
            "maxConnectionStringTtlInDays": 1,
        }))
        .unwrap()
    }

    fn service(recipient_policy: &str) -> Service {
        serde_json::from_value(serde_json::json!({
            "name": SENDER_TAG,
            "host": "localhost:8080",
            "protocol": "http",
            "routes": [],
            "healthCheckPath": "/health",
            "notifications": {
                "senderTag": SENDER_TAG,
                "displayName": "Service 01",
                "issuerRoles": [ISSUER_ROLE],
                "recipientPolicy": recipient_policy,
                "templates": ["welcome"],
            },
        }))
        .unwrap()
    }

    fn owner(is_principal: bool) -> Owner {
        Owner {
            id: Uuid::new_v4(),
            email: OWNER_EMAIL.to_string(),
            first_name: None,
            last_name: None,
            username: None,
            is_principal,
        }
    }

    fn issuer(role: &str, tenant_id: Uuid, is_principal: bool) -> Profile {
        Profile::new(
            vec![owner(is_principal)],
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
            Some(LicensedResources::Records(vec![LicensedResource {
                acc_id: Uuid::new_v4(),
                tenant_id,
                role_id: Uuid::new_v4(),
                acc_name: "Account".to_string(),
                sys_acc: false,
                role: role.to_string(),
                perm: Permission::Write,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            }])),
            None,
        )
    }

    fn services_with(services: Vec<Service>) -> MockServiceRead {
        let mut service_repo = MockServiceRead::new();
        service_repo
            .expect_list_services()
            .returning(move |_, _, _| {
                Ok(FetchManyResponseKind::Found(services.clone()))
            });

        service_repo
    }

    fn untouched_repos() -> (
        MockTokenRegistration,
        MockLocalMessageWrite,
        MockTenantFetching,
    ) {
        let mut token_repo = MockTokenRegistration::new();
        token_repo.expect_create_connection_string().never();

        let mut message_repo = MockLocalMessageWrite::new();
        message_repo.expect_send().never();

        (token_repo, message_repo, MockTenantFetching::new())
    }

    /// Run the use case expecting a rejection before anything is stored.
    async fn rejected(
        profile: Profile,
        expiration: i64,
        tenant_id: Option<Uuid>,
        sender_tag: &str,
        services: Vec<Service>,
    ) -> MappedErrors {
        let service_repo = services_with(services);
        let (token_repo, message_repo, tenant_repo) = untouched_repos();

        create_notification_connection_string(
            profile,
            "token".to_string(),
            expiration,
            tenant_id,
            sender_tag.to_string(),
            notifications_settings(),
            life_cycle(),
            Box::new(&service_repo),
            Box::new(&token_repo),
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await
        .unwrap_err()
    }

    // ? -----------------------------------------------------------------------
    // ? Happy path
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn issues_a_scoped_string_and_emails_the_owner(
    ) -> Result<(), MappedErrors> {
        let tenant_id = Uuid::new_v4();
        let service_repo = services_with(vec![service("owner")]);

        let mut token_repo = MockTokenRegistration::new();
        token_repo
            .expect_create_connection_string()
            .times(1)
            .returning(|meta, expires| {
                Ok(CreateResponseKind::Created(Token::new(
                    None,
                    expires,
                    MultiTypeMeta::UserAccountConnectionString(meta),
                )))
            });

        let mut message_repo = MockLocalMessageWrite::new();
        message_repo
            .expect_send()
            .withf(|event| event.message.to.email() == OWNER_EMAIL)
            .times(1)
            .returning(|_| Ok(CreateResponseKind::Created(None)));

        let mut tenant_repo = MockTenantFetching::new();
        tenant_repo.expect_get_tenant_public_by_id().returning(|_| {
            Ok(mycelium_base::entities::FetchResponseKind::NotFound(None))
        });

        let issued = create_notification_connection_string(
            issuer(ISSUER_ROLE, tenant_id, true),
            "token".to_string(),
            ONE_HOUR,
            Some(tenant_id),
            SENDER_TAG.to_string(),
            notifications_settings(),
            life_cycle(),
            Box::new(&service_repo),
            Box::new(&token_repo),
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await?;

        let scope = UserAccountScope::try_from(issued).unwrap();

        scope.verify_signature(&life_cycle()).await?;

        assert_eq!(
            scope.get_scopes(),
            Some(vec![NotificationScope::NotifyEmail])
        );
        assert_eq!(scope.get_sender_tag(), Some(SENDER_TAG.to_string()));
        assert_eq!(scope.get_tenant_id(), Some(tenant_id));

        Ok(())
    }

    // ? -----------------------------------------------------------------------
    // ? Rejections
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn profile_without_principal_owner_is_rejected() {
        let tenant_id = Uuid::new_v4();
        let error = rejected(
            issuer(ISSUER_ROLE, tenant_id, false),
            ONE_HOUR,
            Some(tenant_id),
            SENDER_TAG,
            vec![service("owner")],
        )
        .await;

        assert!(error.is_in(vec![NativeErrorCodes::MYC00013]), "{error}");
    }

    #[tokio::test]
    async fn expiration_out_of_bounds_is_rejected() {
        let tenant_id = Uuid::new_v4();

        for expiration in [0, -1, SECONDS_PER_DAY + 1] {
            let error = rejected(
                issuer(ISSUER_ROLE, tenant_id, true),
                expiration,
                Some(tenant_id),
                SENDER_TAG,
                vec![service("owner")],
            )
            .await;

            assert!(error.is_in(vec![NativeErrorCodes::MYC00035]), "{error}");
        }
    }

    #[tokio::test]
    async fn unknown_or_disabled_service_is_rejected() {
        let tenant_id = Uuid::new_v4();
        let mut disabled_service = service("owner");
        disabled_service.notifications = None;

        for (sender_tag, services) in [
            ("service-02", vec![service("owner")]),
            (SENDER_TAG, vec![disabled_service]),
            (SENDER_TAG, vec![]),
        ] {
            let error = rejected(
                issuer(ISSUER_ROLE, tenant_id, true),
                ONE_HOUR,
                Some(tenant_id),
                sender_tag,
                services,
            )
            .await;

            assert!(error.is_in(vec![NativeErrorCodes::MYC00036]), "{error}");
        }
    }

    #[tokio::test]
    async fn tenant_members_policy_without_tenant_is_rejected() {
        let error = rejected(
            issuer(ISSUER_ROLE, Uuid::new_v4(), true),
            ONE_HOUR,
            None,
            SENDER_TAG,
            vec![service("tenantMembers")],
        )
        .await;

        assert!(error.is_in(vec![NativeErrorCodes::MYC00035]), "{error}");
    }

    #[tokio::test]
    async fn issuer_without_the_service_role_is_rejected() {
        let tenant_id = Uuid::new_v4();
        let error = rejected(
            issuer("other-role", tenant_id, true),
            ONE_HOUR,
            Some(tenant_id),
            SENDER_TAG,
            vec![service("owner")],
        )
        .await;

        assert!(error.is_in(vec![NativeErrorCodes::MYC00037]), "{error}");
    }
}
