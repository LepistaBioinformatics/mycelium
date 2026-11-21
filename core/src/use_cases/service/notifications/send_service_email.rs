use super::{
    authorize_service_sender, build_service_messages, check_recipient_policy,
    parse_notification_recipients, render_service_email, validate_send_request,
    ServiceEmailDraft,
};
use crate::{
    domain::{
        dtos::{
            notification_dispatch::{NotificationDispatchIssuer, QuotaOutcome},
            send_service_email_request::SendServiceEmailRequest,
            token::UserAccountConnectionString,
        },
        entities::{
            LicensedResourcesFetching, NotificationDispatchRegistration,
            ProfileFetching, ServiceRead, TenantFetching,
        },
    },
    models::{AccountLifeCycle, ServiceNotificationsConfig, ServiceTemplates},
};

use mycelium_base::utils::errors::MappedErrors;

/// Render, trace and queue an email sent by a downstream service
///
/// `connection_string` must be a verified, stored notification string
/// (`notify:email` scope) and `connection_string_id` its `token.id`. The
/// service is taken from the string `SVC` bean, never from the request. In
/// order:
///
/// 1. resolve the service, which must still enable notifications (MYC00036);
/// 2. re-check the string creator on a fresh profile (MYC00037);
/// 3. validate the request and parse the recipients (MYC00038);
/// 4. apply the service recipient policy (MYC00039);
/// 5. render the template, then append the attribution footer;
/// 6. build one message and one dispatch row per recipient;
/// 7. register and queue the whole batch within the hourly quota.
///
/// The quota answer is returned as is: `QuotaOutcome::Exceeded` carries the
/// `retry_after_secs` the caller needs to answer the client.
#[tracing::instrument(name = "send_service_email", skip_all)]
pub async fn send_service_email(
    connection_string: UserAccountConnectionString,
    connection_string_id: i32,
    request: SendServiceEmailRequest,
    notifications_settings: &ServiceNotificationsConfig,
    service_templates: &ServiceTemplates,
    life_cycle: &AccountLifeCycle,
    service_read_repo: Box<&dyn ServiceRead>,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    notification_dispatch_repo: Box<&dyn NotificationDispatchRegistration>,
) -> Result<QuotaOutcome, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Authorize the service and the string creator, as of now
    // ? -----------------------------------------------------------------------

    let (config, profile) = authorize_service_sender(
        &connection_string,
        service_read_repo,
        profile_fetching_repo,
        Box::new(*licensed_resources_fetching_repo),
    )
    .await?;

    let tenant_id = connection_string.scope.get_tenant_id();

    // ? -----------------------------------------------------------------------
    // ? Validate the request and its recipients
    // ? -----------------------------------------------------------------------

    validate_send_request(&request, &config)?;

    let recipients = parse_notification_recipients(&request.to)?;

    check_recipient_policy(
        &config,
        &profile,
        tenant_id,
        &recipients,
        licensed_resources_fetching_repo,
    )
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Render, build and queue one message per recipient
    // ? -----------------------------------------------------------------------

    let rendered = render_service_email(
        &config,
        &request,
        tenant_id,
        service_templates,
        life_cycle,
        tenant_fetching_repo,
    )
    .await?;

    let draft = ServiceEmailDraft {
        config: &config,
        issuer: NotificationDispatchIssuer {
            connection_string_id,
            account_id: connection_string.account_id,
            tenant_id,
        },
        template: request.template,
        rendered,
    };

    let (rows, messages) = build_service_messages(
        &draft,
        recipients,
        notifications_settings.sender_tag_mode,
        life_cycle,
    )
    .await?;

    notification_dispatch_repo
        .register_and_enqueue(
            &config.sender_tag,
            config.quota_per_hour,
            rows,
            messages,
        )
        .await
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
                email::Email,
                guest_role::Permission,
                message::{FromEmail, MessageSendingEvent},
                native_error_codes::NativeErrorCodes,
                notification_dispatch::NewNotificationDispatch,
                profile::{LicensedResource, Owner, Profile},
                service::Service,
                token::{NotificationScope, UserAccountScope},
            },
            entities::{
                MockLicensedResourcesFetching,
                MockNotificationDispatchRegistration, MockServiceRead,
                MockTenantFetching,
            },
        },
        models::{HmacSecretEntry, HmacSecretSet},
        use_cases::service::notifications::{
            MESSAGE_ID_HEADER, SOURCE_SERVICE_HEADER,
        },
    };

    use async_trait::async_trait;
    use chrono::{Duration, Local};
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::{FetchManyResponseKind, FetchResponseKind};
    use serde_json::json;
    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    const SENDER_TAG: &str = "service-01";
    const ISSUER_ROLE: &str = "service-01-admin";
    const OWNER_EMAIL: &str = "owner@example.com";

    type Batch = (Vec<NewNotificationDispatch>, Vec<MessageSendingEvent>);

    // ? -----------------------------------------------------------------------
    // ? Fixtures
    // ? -----------------------------------------------------------------------

    fn life_cycle() -> AccountLifeCycle {
        let secret = |value: &str| SecretResolver::Value(value.to_string());

        AccountLifeCycle {
            domain_name: secret("Test Domain"),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: Some(secret("Mycelium")),
            noreply_email: secret("noreply@example.com"),
            support_name: None,
            support_email: secret("support@example.com"),
            token_secret: secret("test-secret"),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: secret("test-hmac"),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    struct TemplatesDir(PathBuf);

    impl TemplatesDir {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("myc-send-service-email-{}", Uuid::new_v4()));
            let service = root.join(SENDER_TAG).join("en-us");

            fs::create_dir_all(&service).unwrap();
            fs::write(service.join("welcome.jinja"), "<p>Hi {{ name }}</p>")
                .unwrap();
            fs::write(service.join("welcome.subject"), "Welcome {{ name }}")
                .unwrap();

            Self(root)
        }

        fn load(&self) -> ServiceTemplates {
            ServiceTemplates::load(&self.0, "en-us").unwrap()
        }
    }

    impl Drop for TemplatesDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn service(policy: &str) -> Service {
        serde_json::from_value(json!({
            "name": SENDER_TAG,
            "host": "localhost:8080",
            "protocol": "http",
            "routes": [],
            "healthCheckPath": "/health",
            "notifications": {
                "senderTag": SENDER_TAG,
                "displayName": "Service 01",
                "issuerRoles": [ISSUER_ROLE],
                "recipientPolicy": policy,
                "templates": ["welcome"],
                "quotaPerHour": 50,
                "maxRecipientsPerMessage": 2,
            },
        }))
        .unwrap()
    }

    fn services_with(services: Vec<Service>) -> MockServiceRead {
        let mut repo = MockServiceRead::new();
        repo.expect_list_services().returning(move |_, _, _| {
            Ok(FetchManyResponseKind::Found(services.clone()))
        });
        repo
    }

    fn issuer_profile(account_id: Uuid, account_is_active: bool) -> Profile {
        Profile::new(
            vec![Owner {
                id: Uuid::new_v4(),
                email: OWNER_EMAIL.to_string(),
                first_name: None,
                last_name: None,
                username: None,
                is_principal: true,
            }],
            account_id,
            false,
            false,
            false,
            true,
            account_is_active,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    /// `ProfileFetching` has no generated mock: answer with a fixed profile.
    struct FreshProfile(Option<Profile>);

    #[async_trait]
    impl ProfileFetching for FreshProfile {
        async fn get_from_email(
            &self,
            _: Email,
        ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
            match self.0.to_owned() {
                Some(profile) => Ok(FetchResponseKind::Found(profile)),
                None => Ok(FetchResponseKind::NotFound(None)),
            }
        }

        async fn get_from_token(
            &self,
            _: String,
        ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
            unimplemented!()
        }
    }

    /// Licences of the creator, as read for the fresh profile.
    fn licences_with_role(
        role: &'static str,
        tenant_id: Uuid,
    ) -> MockLicensedResourcesFetching {
        let mut repo = MockLicensedResourcesFetching::new();

        repo.expect_list_licensed_resources().returning(
            move |_, _, _, _, _| {
                Ok(FetchManyResponseKind::Found(vec![LicensedResource {
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
                }]))
            },
        );
        repo.expect_list_tenants_ownership()
            .returning(|_, _| Ok(FetchManyResponseKind::NotFound));

        repo
    }

    fn tenants() -> MockTenantFetching {
        let mut repo = MockTenantFetching::new();
        repo.expect_get_tenant_public_by_id()
            .returning(|_| Ok(FetchResponseKind::NotFound(None)));
        repo
    }

    /// A dispatch repository recording the batch it was given.
    fn recording_dispatch(
        outcome: QuotaOutcome,
    ) -> (
        MockNotificationDispatchRegistration,
        Arc<Mutex<Option<Batch>>>,
    ) {
        let recorded = Arc::new(Mutex::new(None));
        let sink = recorded.clone();
        let mut repo = MockNotificationDispatchRegistration::new();

        repo.expect_register_and_enqueue()
            .withf(|tag, quota, _, _| tag == SENDER_TAG && *quota == 50)
            .returning(move |_, _, rows, messages| {
                *sink.lock().unwrap() = Some((rows, messages));
                Ok(outcome.clone())
            });

        (repo, recorded)
    }

    fn untouched_dispatch() -> MockNotificationDispatchRegistration {
        let mut repo = MockNotificationDispatchRegistration::new();
        repo.expect_register_and_enqueue().never();
        repo
    }

    async fn connection_string(
        account_id: Uuid,
        tenant_id: Uuid,
    ) -> UserAccountConnectionString {
        let mut scope = UserAccountScope::new_notification(
            account_id,
            Local::now() + Duration::days(1),
            Some(tenant_id),
            vec![NotificationScope::NotifyEmail],
            SENDER_TAG.to_string(),
            life_cycle(),
        )
        .await
        .unwrap();

        UserAccountConnectionString::new_signed_token(
            &mut scope,
            account_id,
            Email::from_string(OWNER_EMAIL.to_string()).unwrap(),
            life_cycle(),
            Some("notifications".to_string()),
        )
        .await
        .unwrap()
    }

    fn request(
        to: &[&str],
        variables: serde_json::Value,
    ) -> SendServiceEmailRequest {
        serde_json::from_value(json!({
            "to": to,
            "template": "welcome",
            "variables": variables,
        }))
        .unwrap()
    }

    /// Everything a send needs; each test changes one piece.
    struct Scenario {
        services: Vec<Service>,
        profile: Option<Profile>,
        role: &'static str,
        request: SendServiceEmailRequest,
        account_id: Uuid,
        tenant_id: Uuid,
    }

    impl Scenario {
        fn new() -> Self {
            let account_id = Uuid::new_v4();

            Self {
                services: vec![service("owner")],
                profile: Some(issuer_profile(account_id, true)),
                role: ISSUER_ROLE,
                request: request(&[OWNER_EMAIL], json!({ "name": "Ana" })),
                account_id,
                tenant_id: Uuid::new_v4(),
            }
        }

        async fn run(
            self,
            dispatch: MockNotificationDispatchRegistration,
        ) -> Result<QuotaOutcome, MappedErrors> {
            let templates = TemplatesDir::new();
            let services = services_with(self.services);
            let profiles = FreshProfile(self.profile);
            let licences = licences_with_role(self.role, self.tenant_id);
            let tenants = tenants();

            send_service_email(
                connection_string(self.account_id, self.tenant_id).await,
                7,
                self.request,
                &ServiceNotificationsConfig::default(),
                &templates.load(),
                &life_cycle(),
                Box::new(&services),
                Box::new(&profiles),
                Box::new(&licences),
                Box::new(&tenants),
                Box::new(&dispatch),
            )
            .await
        }

        async fn rejected(self, code: NativeErrorCodes) {
            let error = self.run(untouched_dispatch()).await.unwrap_err();

            assert!(error.is_in(vec![code]), "{error}");
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Happy path
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn queues_one_traced_message_per_recipient(
    ) -> Result<(), MappedErrors> {
        let mut scenario = Scenario::new();
        scenario.services = vec![service("any")];
        scenario.request = request(
            &[OWNER_EMAIL, "second@example.com"],
            json!({ "name": "<b>Ana</b>" }),
        );
        let account_id = scenario.account_id;
        let tenant_id = scenario.tenant_id;

        let (dispatch, recorded) =
            recording_dispatch(QuotaOutcome::Accepted(vec![]));

        scenario.run(dispatch).await?;

        let (rows, messages) = recorded.lock().unwrap().take().unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(messages.len(), 2);

        for (row, message) in rows.iter().zip(messages.iter()) {
            assert_eq!(row.id, message.id);
            assert_eq!(row.recipient, message.message.to);
            assert_eq!(row.issuer.connection_string_id, 7);
            assert_eq!(row.issuer.account_id, account_id);
            assert_eq!(row.issuer.tenant_id, Some(tenant_id));
            assert_eq!(row.template, "welcome");
            assert_eq!(row.locale, "en-us");

            let FromEmail::NamedEmail(from) = &message.message.from else {
                panic!("expected a named sender");
            };
            assert_eq!(
                from,
                "\"Service 01 via Mycelium\" <noreply@example.com>"
            );

            assert_eq!(
                message.message.subject,
                "Welcome &lt;b&gt;Ana&lt;&#x2F;b&gt;"
            );
            assert!(message.message.body.starts_with("<p>Hi &lt;b&gt;"));
            assert!(message.message.body.contains("Service 01"));
            assert!(message.message.cc.is_none());
            assert!(message.message.headers.contains(&(
                SOURCE_SERVICE_HEADER.to_string(),
                SENDER_TAG.to_string()
            )));
            assert!(message.message.headers.contains(&(
                MESSAGE_ID_HEADER.to_string(),
                message.id.to_string()
            )));
        }

        assert_eq!(messages[0].message.to.email(), OWNER_EMAIL);
        assert_eq!(messages[1].message.to.email(), "second@example.com");
        assert_ne!(rows[0].id, rows[1].id);

        Ok(())
    }

    #[tokio::test]
    async fn exceeded_quota_is_passed_through() -> Result<(), MappedErrors> {
        let exceeded = QuotaOutcome::Exceeded {
            retry_after_secs: 120,
        };
        let (dispatch, _) = recording_dispatch(exceeded.clone());

        assert_eq!(Scenario::new().run(dispatch).await?, exceeded);

        Ok(())
    }

    // ? -----------------------------------------------------------------------
    // ? Authorization at send time
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn rejects_a_service_disabled_since_issuance() {
        let mut disabled = service("owner");
        disabled.notifications = None;

        let mut scenario = Scenario::new();
        scenario.services = vec![disabled];

        scenario.rejected(NativeErrorCodes::MYC00036).await;
    }

    #[tokio::test]
    async fn rejects_a_revoked_issuer_role() {
        let mut scenario = Scenario::new();
        scenario.role = "another-role";

        scenario.rejected(NativeErrorCodes::MYC00037).await;
    }

    #[tokio::test]
    async fn rejects_a_deactivated_creator_account() {
        let mut scenario = Scenario::new();
        scenario.profile = Some(issuer_profile(scenario.account_id, false));

        scenario.rejected(NativeErrorCodes::MYC00037).await;
    }

    #[tokio::test]
    async fn rejects_an_unregistered_creator() {
        let mut scenario = Scenario::new();
        scenario.profile = None;

        scenario.rejected(NativeErrorCodes::MYC00037).await;
    }

    #[tokio::test]
    async fn rejects_an_email_now_owned_by_another_account() {
        let mut scenario = Scenario::new();
        scenario.profile = Some(issuer_profile(Uuid::new_v4(), true));

        scenario.rejected(NativeErrorCodes::MYC00037).await;
    }

    // ? -----------------------------------------------------------------------
    // ? Request validation and recipient policy
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn rejects_a_recipient_count_out_of_range() {
        for to in [vec![], vec![OWNER_EMAIL; 3]] {
            let mut scenario = Scenario::new();
            scenario.request = request(&to, json!({}));

            scenario.rejected(NativeErrorCodes::MYC00038).await;
        }
    }

    #[tokio::test]
    async fn rejects_an_undeclared_template() {
        let mut scenario = Scenario::new();
        scenario.request.template = "invoice-ready".to_string();

        scenario.rejected(NativeErrorCodes::MYC00038).await;
    }

    #[tokio::test]
    async fn rejects_non_scalar_variables() {
        let mut scenario = Scenario::new();
        scenario.request = request(&[OWNER_EMAIL], json!({ "a": [1, 2] }));

        scenario.rejected(NativeErrorCodes::MYC00038).await;
    }

    #[tokio::test]
    async fn rejects_variables_over_8_kib() {
        let mut scenario = Scenario::new();
        scenario.request =
            request(&[OWNER_EMAIL], json!({ "v": "x".repeat(8 * 1024) }));

        scenario.rejected(NativeErrorCodes::MYC00038).await;
    }

    #[tokio::test]
    async fn rejects_an_injected_recipient() {
        let mut scenario = Scenario::new();
        scenario.services = vec![service("any")];
        scenario.request = request(
            &["victim@allowed.example\r\nBcc: other@evil.example"],
            json!({}),
        );

        scenario.rejected(NativeErrorCodes::MYC00038).await;
    }

    #[tokio::test]
    async fn rejects_a_recipient_policy_violation() {
        let mut scenario = Scenario::new();
        scenario.request = request(&["stranger@example.com"], json!({}));

        scenario.rejected(NativeErrorCodes::MYC00039).await;
    }

    #[tokio::test]
    async fn rejects_a_missing_template_variable() {
        let mut scenario = Scenario::new();
        scenario.request = request(&[OWNER_EMAIL], json!({}));

        scenario.rejected(NativeErrorCodes::MYC00038).await;
    }
}
