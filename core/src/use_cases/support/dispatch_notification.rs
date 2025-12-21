use crate::{
    domain::{
        dtos::{
            email::Email,
            message::{FromEmail, Message, MessageSendingEvent},
            tenant::TenantMetaKey,
        },
        entities::{LocalMessageWrite, TenantFetching},
    },
    models::AccountLifeCycle,
    settings::{DEFAULT_TENANT_ID_KEY, TEMPLATES},
};

use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;
use uuid::Uuid;

#[tracing::instrument(name = "dispatch_notification", skip_all)]
pub(crate) async fn dispatch_notification<T: ToString>(
    parameters: Vec<(T, String)>,
    template_path_prefix: T,
    config: AccountLifeCycle,
    to: Email,
    cc: Option<Email>,
    local_message_write_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
    tracing::info!("Dispatching notification");

    let (context, locale) =
        populate_tenant_info(&parameters, &config, tenant_fetching_repo)
            .await?;

    let locale = if let Some(locale) = locale {
        //
        // Use the tenant preferred locale if available
        //
        tracing::trace!("Communicating with tenant locale: {:?}", locale);
        locale
    } else if let Some(locale) = config.locale {
        //
        // Use the account locale if available
        //
        tracing::trace!("Communicating with system locale: {:?}", locale);
        locale.async_get_or_error().await?
    } else {
        //
        // Use the default locale if no locale is available
        //
        tracing::trace!("Communicating with default locale: 'en-us'");
        "en-us".to_string()
    };

    //
    // Verify if the selected locale exists in templates folder
    // If not, use the default en-us one
    //
    let verified_locale = {
        let template_path_prefix_str = template_path_prefix.to_string();
        let body_path = format!(
            "{locale}/{prefix}.jinja",
            locale = locale,
            prefix = template_path_prefix_str
        );

        let template_names: Vec<_> = TEMPLATES.get_template_names().collect();

        if template_names.contains(&body_path.as_str()) {
            locale
        } else {
            tracing::warn!(
                "Locale '{}' not found in templates, falling back to 'en-us'",
                locale
            );
            "en-us".to_string()
        }
    };

    let body_path = format!(
        "{locale}/{path}",
        locale = verified_locale,
        path = format!(
            "{prefix}.jinja",
            prefix = template_path_prefix.to_string()
        )
    );

    let body = match TEMPLATES.render(body_path.as_str(), &context) {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!(
                "Unable to render email template: {err}"
            ))
            .as_error();
        }
    };

    let subject_path = format!(
        "{locale}/{path}",
        locale = verified_locale,
        path = format!(
            "{prefix}.subject",
            prefix = template_path_prefix.to_string()
        )
    );

    let subject_ =
        match TEMPLATES.render(subject_path.as_str(), &Context::new()) {
            Ok(res) => res,
            Err(err) => {
                return use_case_err(format!(
                    "Unable to render email subject: {err}"
                ))
                .as_error();
            }
        };

    let from_email =
        Email::from_string(config.noreply_email.async_get_or_error().await?)?;

    let from = if let Some(name) = config.noreply_name {
        FromEmail::NamedEmail(format!(
            "{} <{}>",
            name.async_get_or_error().await?,
            from_email.email()
        ))
    } else {
        FromEmail::Email(from_email)
    };

    local_message_write_repo
        .send(MessageSendingEvent::new(Message {
            from,
            to,
            cc,
            subject: subject_,
            body,
        }))
        .await
}

#[tracing::instrument(name = "populate_tenant_info", skip_all)]
async fn populate_tenant_info<T: ToString>(
    parameters: &Vec<(T, String)>,
    config: &AccountLifeCycle,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<(Context, Option<String>), MappedErrors> {
    let mut context = Context::new();
    let mut optional_locale = None;

    let tenant_found = if let Some((_, tenant_id)) = parameters
        .iter()
        .find(|(key, _)| key.to_string() == DEFAULT_TENANT_ID_KEY)
    {
        if let Ok(tenant_id) = tenant_id.parse::<Uuid>() {
            if let FetchResponseKind::Found(tenant) = tenant_fetching_repo
                .get_tenant_public_by_id(tenant_id)
                .await?
            {
                //
                // Inject the tenant name
                //
                context.insert("domain_name", tenant.name.as_str());

                if let Some(meta) = &tenant.meta {
                    //
                    // Inject the tenant website URL
                    //
                    if let Some(website_url) =
                        meta.get(&TenantMetaKey::WebsiteUrl)
                    {
                        context.insert("domain_url", website_url.as_str());
                    }

                    //
                    // Inject the tenant support email
                    //
                    if let Some(support_email) =
                        meta.get(&TenantMetaKey::SupportEmail)
                    {
                        context.insert("support_email", support_email.as_str());
                    }

                    //
                    // Populate the tenant preferred locale
                    //
                    optional_locale = meta
                        .get(&TenantMetaKey::Locale)
                        .map(|locale| locale.to_owned());
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // If tenant was not found or not provided, use config values as fallback
    if !tenant_found {
        context.insert(
            "domain_name",
            config.domain_name.async_get_or_error().await?.as_str(),
        );

        if let Some(domain_url) = &config.domain_url {
            context.insert(
                "domain_url",
                domain_url.async_get_or_error().await?.as_str(),
            );
        }

        context.insert(
            "support_email",
            &config.support_email.async_get_or_error().await?,
        );
    }

    for (key, value) in parameters {
        context.insert(key.to_string(), &value.to_string());
    }

    Ok((context, optional_locale))
}

// * ---------------------------------------------------------------------------
// * TESTS
// *
// * IMPORTANT: These tests require TEMPLATES_DIR environment variable to be
// * set to an absolute path before running. The TEMPLATES lazy_static is
// * initialized when the module is first loaded, so TEMPLATES_DIR must be set
// * before cargo test runs.
// *
// * Run tests with: TEMPLATES_DIR=/workspaces/mycelium/templates cargo test
// * Or from workspace root: TEMPLATES_DIR=$(pwd)/templates cargo test
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        email::Email,
        profile::Owner,
        tenant::{Tenant, TenantMeta, TenantMetaKey},
    };
    use async_trait::async_trait;
    use chrono::Local;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::{dtos::Children, entities::FetchManyResponseKind};
    use std::collections::HashMap;
    use std::env;

    // ? -----------------------------------------------------------------------
    // ? Setup function to ensure TEMPLATES_DIR is set
    // ? -----------------------------------------------------------------------

    fn setup_templates_dir() {
        if env::var("TEMPLATES_DIR").is_err() {
            // Use current working directory (equivalent to ${PWD}/templates)
            if let Ok(current_dir) = env::current_dir() {
                let templates_path = current_dir.join("templates");
                if let Some(abs_path) = templates_path.to_str() {
                    env::set_var("TEMPLATES_DIR", abs_path);
                    return;
                }
            }

            // Fallback: try CARGO_MANIFEST_DIR (set by cargo test)
            if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
                let templates_path = format!("{}/templates", manifest_dir);
                env::set_var("TEMPLATES_DIR", &templates_path);
                return;
            }

            // Last resort: use current directory (equivalent to ${PWD}/templates)
            if let Ok(pwd) = env::current_dir() {
                if let Some(pwd_str) = pwd.to_str() {
                    env::set_var(
                        "TEMPLATES_DIR",
                        &format!("{}/templates", pwd_str),
                    );
                }
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Mock repositories
    // ? -----------------------------------------------------------------------

    struct MockLocalMessageWrite {
        should_fail: bool,
        message_id: Option<Uuid>,
    }

    impl MockLocalMessageWrite {
        fn new() -> Self {
            Self {
                should_fail: false,
                message_id: Some(Uuid::new_v4()),
            }
        }

        fn with_error() -> Self {
            Self {
                should_fail: true,
                message_id: None,
            }
        }
    }

    #[async_trait]
    impl LocalMessageWrite for MockLocalMessageWrite {
        async fn send(
            &self,
            _message_event: MessageSendingEvent,
        ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
            if self.should_fail {
                return use_case_err("Failed to send message".to_string())
                    .as_error();
            }

            Ok(CreateResponseKind::Created(self.message_id))
        }

        async fn update_message_event(
            &self,
            _message_event: MessageSendingEvent,
        ) -> Result<(), MappedErrors> {
            unimplemented!()
        }

        async fn delete_message_event(
            &self,
            _id: Uuid,
        ) -> Result<(), MappedErrors> {
            unimplemented!()
        }

        async fn ping(&self) -> Result<(), MappedErrors> {
            unimplemented!()
        }
    }

    struct MockTenantFetching {
        tenant: Option<Tenant>,
        should_fail: bool,
    }

    impl MockTenantFetching {
        fn with_tenant(tenant: Tenant) -> Self {
            Self {
                tenant: Some(tenant),
                should_fail: false,
            }
        }

        fn not_found() -> Self {
            Self {
                tenant: None,
                should_fail: false,
            }
        }

        fn with_error() -> Self {
            Self {
                tenant: None,
                should_fail: true,
            }
        }
    }

    #[async_trait]
    impl TenantFetching for MockTenantFetching {
        async fn get_tenant_owned_by_me(
            &self,
            _id: Uuid,
            _owners_ids: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_tenant_public_by_id(
            &self,
            _id: Uuid,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            if self.should_fail {
                return use_case_err("Failed to fetch tenant".to_string())
                    .as_error();
            }

            match &self.tenant {
                Some(tenant) => Ok(FetchResponseKind::Found(tenant.clone())),
                None => Ok(FetchResponseKind::NotFound(Some(
                    "tenant_id".to_string(),
                ))),
            }
        }

        async fn get_tenants_by_manager_account(
            &self,
            _id: Uuid,
            _manager_ids: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn filter_tenants_as_manager(
            &self,
            _name: Option<String>,
            _owner: Option<Uuid>,
            _metadata: Option<(TenantMetaKey, String)>,
            _tag: Option<(String, String)>,
            _page_size: Option<i32>,
            _skip: Option<i32>,
        ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
            unimplemented!()
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Helper functions
    // ? -----------------------------------------------------------------------

    fn create_test_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("Test Domain".to_string()),
            domain_url: Some(SecretResolver::Value(
                "https://test.com".to_string(),
            )),
            locale: Some(SecretResolver::Value("en-us".to_string())),
            token_expiration: SecretResolver::Value(3600),
            noreply_name: Some(SecretResolver::Value(
                "Test System".to_string(),
            )),
            noreply_email: SecretResolver::Value(
                "noreply@test.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@test.com".to_string(),
            ),
            token_secret: SecretResolver::Value("test-secret".to_string()),
        }
    }

    fn create_test_email() -> Email {
        Email::from_string("test@example.com".to_string()).unwrap()
    }

    fn create_test_tenant_with_meta(meta: Option<TenantMeta>) -> Tenant {
        let owner = Owner {
            id: Uuid::new_v4(),
            email: "owner@test.com".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("Owner".to_string()),
            username: Some("testowner".to_string()),
            is_principal: true,
        };

        Tenant {
            id: Some(Uuid::new_v4()),
            name: "Test Tenant".to_string(),
            description: Some("Test tenant description".to_string()),
            owners: Children::Records(vec![owner]),
            manager: None,
            tags: None,
            meta,
            status: None,
            created: Local::now(),
            updated: None,
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test cases
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_dispatch_notification_with_tenant_id_and_meta() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let mut meta = HashMap::new();
        meta.insert(
            TenantMetaKey::WebsiteUrl,
            "https://tenant.example.com".to_string(),
        );
        meta.insert(
            TenantMetaKey::SupportEmail,
            "support@tenant.example.com".to_string(),
        );
        meta.insert(TenantMetaKey::Locale, "en-us".to_string()); // Use en-us for testing

        let tenant = create_test_tenant_with_meta(Some(meta));
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::with_tenant(tenant.clone());

        let result = dispatch_notification(
            vec![
                (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
                ("verification_code", "123456".to_string()),
            ],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(id)) => assert!(id.is_some()),
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_without_tenant_id() {
        setup_templates_dir();
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::not_found();

        let result = dispatch_notification(
            vec![("verification_code", "123456".to_string())],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(id)) => assert!(id.is_some()),
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_tenant_not_found() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::not_found();

        let result = dispatch_notification(
            vec![
                (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
                ("verification_code", "123456".to_string()),
            ],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        // Should still succeed, but use config values instead of tenant values
        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_locale_existing() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let mut meta = HashMap::new();
        meta.insert(TenantMetaKey::Locale, "en-us".to_string()); // Use en-us for testing

        let tenant = create_test_tenant_with_meta(Some(meta));
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::with_tenant(tenant);

        let result = dispatch_notification(
            vec![
                (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
                ("verification_code", "123456".to_string()),
            ],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_locale_fallback() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let mut meta = HashMap::new();
        // Use a locale that doesn't exist in templates (e.g., "fr-fr")
        meta.insert(TenantMetaKey::Locale, "fr-fr".to_string());

        let tenant = create_test_tenant_with_meta(Some(meta));
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::with_tenant(tenant);

        let result = dispatch_notification(
            vec![
                (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
                ("verification_code", "123456".to_string()),
            ],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        // Should succeed with fallback to en-us
        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_tenant_without_meta() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let tenant = create_test_tenant_with_meta(None);
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::with_tenant(tenant);

        let result = dispatch_notification(
            vec![
                (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
                ("verification_code", "123456".to_string()),
            ],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_send_error() {
        setup_templates_dir();
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::with_error();
        let tenant_repo = MockTenantFetching::not_found();

        let result = dispatch_notification(
            vec![("verification_code", "123456".to_string())],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatch_notification_with_cc() {
        setup_templates_dir();
        let config = create_test_config();
        let email = create_test_email();
        let cc_email =
            Email::from_string("cc@example.com".to_string()).unwrap();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::not_found();

        let result = dispatch_notification(
            vec![("verification_code", "123456".to_string())],
            "email/activation-code",
            config,
            email,
            Some(cc_email),
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_with_config_locale() {
        setup_templates_dir();
        let mut config = create_test_config();
        config.locale = Some(SecretResolver::Value("es".to_string()));

        let email = create_test_email();
        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::not_found();

        let result = dispatch_notification(
            vec![("verification_code", "123456".to_string())],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_dispatch_notification_tenant_fetching_error() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::with_error();

        let result = dispatch_notification(
            vec![(DEFAULT_TENANT_ID_KEY, tenant_id.to_string())],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatch_notification_with_tenant_partial_meta() {
        setup_templates_dir();
        let tenant_id = Uuid::new_v4();
        let mut meta = HashMap::new();
        // Only website URL, no support email or locale
        meta.insert(
            TenantMetaKey::WebsiteUrl,
            "https://tenant.example.com".to_string(),
        );

        let tenant = create_test_tenant_with_meta(Some(meta));
        let config = create_test_config();
        let email = create_test_email();

        let message_repo = MockLocalMessageWrite::new();
        let tenant_repo = MockTenantFetching::with_tenant(tenant);

        let result = dispatch_notification(
            vec![
                (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
                ("verification_code", "123456".to_string()),
            ],
            "email/activation-code",
            config,
            email,
            None,
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        match result {
            Ok(CreateResponseKind::Created(_)) => {}
            Ok(_) => panic!("Expected Created response"),
            Err(err) => panic!("Expected success but got error: {:?}", err),
        }
    }
}
