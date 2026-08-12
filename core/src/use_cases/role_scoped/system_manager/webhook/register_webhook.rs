use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            http::HttpMethod,
            http_secret::HttpSecret,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            webhook::{WebHook, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{
            EncryptionKeyFetching, ResourceAuditLogRegistration,
            WebHookRegistration,
        },
        utils::{build_aad, AAD_FIELD_HTTP_SECRET},
    },
    models::AccountLifeCycle,
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "register_webhook",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn register_webhook(
    profile: Profile,
    name: String,
    description: Option<String>,
    url: String,
    trigger: WebHookTrigger,
    method: Option<HttpMethod>,
    secret: Option<HttpSecret>,
    config: AccountLifeCycle,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    encryption_key_fetching_repo: Box<&dyn EncryptionKeyFetching>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch the system DEK (webhooks are global, no tenant)
    // ? -----------------------------------------------------------------------

    let kek = config.derive_kek_bytes().await?;
    let dek = encryption_key_fetching_repo
        .get_or_provision_dek(None, &kek)
        .await?;

    let aad = build_aad(None, AAD_FIELD_HTTP_SECRET);

    // ? -----------------------------------------------------------------------
    // ? Register webhook
    // ? -----------------------------------------------------------------------

    let webhook = WebHook::new_encrypted(
        name,
        description,
        url,
        trigger,
        method,
        secret,
        &dek,
        &aad,
        Some(WrittenBy::new_from_account(profile.acc_id)),
    )?;

    let response = webhook_registration_repo.create(webhook).await?;

    if let CreateResponseKind::Created(ref webhook) = response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Webhook,
            webhook.id.unwrap_or_default(),
            None,
            ResourceAuditEventKind::Created,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "register_webhook" }),
        )
        .await;
    }

    Ok(response)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            profile::{Owner, Profile},
            resource_audit_log::NewResourceAuditLogEvent,
        },
        entities::MockResourceAuditLogRegistration,
    };
    use crate::models::{HmacSecretEntry, HmacSecretSet};

    use async_trait::async_trait;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::utils::errors::use_case_err;
    use shaku::Component;
    use std::str::FromStr;
    use uuid::Uuid;

    #[derive(Component)]
    #[shaku(interface = WebHookRegistration)]
    struct MockWebHookRegistrationRepo {
        pub generate_error: bool,
    }

    #[async_trait]
    impl WebHookRegistration for MockWebHookRegistrationRepo {
        async fn create(
            &self,
            webhook: WebHook,
        ) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
            match self.generate_error {
                true => {
                    return use_case_err("Error while creating webhook.")
                        .as_error()
                }
                false => {
                    let mut webhook = webhook;
                    webhook.id = Some(Uuid::new_v4());
                    Ok(CreateResponseKind::Created(webhook))
                }
            }
        }

        async fn register_execution_event(
            &self,
            _: crate::domain::dtos::webhook::WebHookPayloadArtifact,
        ) -> Result<CreateResponseKind<Uuid>, MappedErrors> {
            unimplemented!()
        }
    }

    #[derive(Component)]
    #[shaku(interface = EncryptionKeyFetching)]
    struct MockEncryptionKeyFetchingRepo;

    #[async_trait]
    impl EncryptionKeyFetching for MockEncryptionKeyFetchingRepo {
        async fn get_or_provision_dek(
            &self,
            _: Option<Uuid>,
            _: &[u8; 32],
        ) -> Result<[u8; 32], MappedErrors> {
            Ok([0u8; 32])
        }
    }

    fn system_manager_profile() -> Profile {
        Profile::new(
            vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "username@domain.com".to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
                is_principal: true,
            }],
            Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0").unwrap(),
            false,
            true,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn test_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("example.com".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: None,
            noreply_email: SecretResolver::Value(
                "noreply@example.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@example.com".to_string(),
            ),
            token_secret: SecretResolver::Value(Uuid::new_v4().to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("test-hmac".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    #[tokio::test]
    async fn register_webhook_emits_audit_event_on_success() {
        let mut mock_audit_repo = MockResourceAuditLogRegistration::new();
        mock_audit_repo
            .expect_create()
            .times(1)
            .withf(move |event: &NewResourceAuditLogEvent| {
                event.resource_type == ResourceAuditResourceType::Webhook
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Created
            })
            .returning(|_| Ok(()));

        let response = register_webhook(
            system_manager_profile(),
            "webhook".to_string(),
            None,
            "https://example.com".to_string(),
            WebHookTrigger::SubscriptionAccountCreated,
            None,
            None,
            test_config(),
            Box::new(&MockWebHookRegistrationRepo {
                generate_error: false,
            }),
            Box::new(&MockEncryptionKeyFetchingRepo),
            Box::new(&mock_audit_repo),
        )
        .await
        .unwrap();

        assert!(matches!(response, CreateResponseKind::Created(_)));
    }

    #[tokio::test]
    async fn register_webhook_does_not_emit_audit_event_on_error() {
        let mut mock_audit_repo = MockResourceAuditLogRegistration::new();
        mock_audit_repo.expect_create().times(0);

        let response = register_webhook(
            system_manager_profile(),
            "webhook".to_string(),
            None,
            "https://example.com".to_string(),
            WebHookTrigger::SubscriptionAccountCreated,
            None,
            None,
            test_config(),
            Box::new(&MockWebHookRegistrationRepo {
                generate_error: true,
            }),
            Box::new(&MockEncryptionKeyFetchingRepo),
            Box::new(&mock_audit_repo),
        )
        .await;

        assert!(response.is_err());
    }
}
