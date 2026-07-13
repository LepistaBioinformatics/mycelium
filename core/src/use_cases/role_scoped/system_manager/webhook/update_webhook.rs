use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            http_secret::HttpSecret,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            resource_audit_log::{
                ResourceAuditEventKind, ResourceAuditResourceType,
            },
            webhook::WebHook,
            written_by::WrittenBy,
        },
        entities::{
            EncryptionKeyFetching, ResourceAuditLogRegistration,
            WebHookFetching, WebHookUpdating,
        },
        utils::{build_aad, AAD_FIELD_HTTP_SECRET},
    },
    models::AccountLifeCycle,
    use_cases::shared::audit::emit_resource_audit_event,
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_webhook",
    fields(profile_id = %profile.acc_id),
    skip(
        profile,
        name,
        description,
        secret,
        config,
        webhook_fetching_repo,
        webhook_updating_repo,
        encryption_key_fetching_repo,
        audit_repo,
    ),
)]
pub async fn update_webhook(
    profile: Profile,
    webhook_id: Uuid,
    name: Option<String>,
    description: Option<String>,
    secret: Option<HttpSecret>,
    config: AccountLifeCycle,
    is_active: Option<bool>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
    webhook_updating_repo: Box<&dyn WebHookUpdating>,
    encryption_key_fetching_repo: Box<&dyn EncryptionKeyFetching>,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch webhook
    // ? -----------------------------------------------------------------------

    let mut webhook = match webhook_fetching_repo.get(webhook_id).await? {
        FetchResponseKind::Found(webhook) => webhook,
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "WebHook with id {} not found.",
                webhook_id
            ))
            .with_code(NativeErrorCodes::MYC00018)
            .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update webhook
    // ? -----------------------------------------------------------------------

    if let Some(name) = name {
        webhook.name = name;
    }

    if let Some(description) = description {
        webhook.description = Some(description);
    }

    if let Some(secret) = secret {
        let kek = config.derive_kek_bytes().await?;
        let dek = encryption_key_fetching_repo
            .get_or_provision_dek(None, &kek)
            .await?;
        let aad = build_aad(None, AAD_FIELD_HTTP_SECRET);
        webhook.set_secret(
            secret,
            &dek,
            &aad,
            Some(WrittenBy::new_from_account(profile.acc_id)),
        )?;
    }

    if let Some(is_active) = is_active {
        webhook.is_active = is_active;
    }

    let response = webhook_updating_repo.update(webhook).await?;

    if let UpdatingResponseKind::Updated(ref webhook) = response {
        emit_resource_audit_event(
            audit_repo,
            ResourceAuditResourceType::Webhook,
            webhook.id.unwrap_or_default(),
            None,
            ResourceAuditEventKind::Updated,
            WrittenBy::new_from_account(profile.acc_id),
            serde_json::json!({ "action": "update_webhook" }),
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
            webhook::WebHookTrigger,
        },
        entities::MockResourceAuditLogRegistration,
    };
    use crate::models::{HmacSecretEntry, HmacSecretSet};

    use async_trait::async_trait;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::FetchResponseKind;
    use shaku::Component;
    use std::str::FromStr;

    #[derive(Component)]
    #[shaku(interface = WebHookFetching)]
    struct MockWebHookFetchingRepo;

    #[async_trait]
    impl WebHookFetching for MockWebHookFetchingRepo {
        async fn get(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::Found(WebHook::new(
                "webhook".to_string(),
                None,
                "https://example.com".to_string(),
                WebHookTrigger::SubscriptionAccountCreated,
                None,
                None,
                None,
            )))
        }

        async fn list(
            &self,
            _: Option<String>,
            _: Option<WebHookTrigger>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<WebHook>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn list_by_trigger(
            &self,
            _: WebHookTrigger,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<WebHook>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn fetch_execution_event(
            &self,
            _: u32,
            _: u32,
            _: Option<
                Vec<crate::domain::dtos::webhook::WebHookExecutionStatus>,
            >,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<
                crate::domain::dtos::webhook::WebHookPayloadArtifact,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    #[derive(Component)]
    #[shaku(interface = WebHookUpdating)]
    struct MockWebHookUpdatingRepo;

    #[async_trait]
    impl WebHookUpdating for MockWebHookUpdatingRepo {
        async fn update(
            &self,
            webhook: WebHook,
        ) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
            let mut webhook = webhook;
            webhook.id = Some(Uuid::new_v4());
            Ok(UpdatingResponseKind::Updated(webhook))
        }

        async fn update_execution_event(
            &self,
            _: crate::domain::dtos::webhook::WebHookPayloadArtifact,
        ) -> Result<
            UpdatingResponseKind<
                crate::domain::dtos::webhook::WebHookPayloadArtifact,
            >,
            MappedErrors,
        > {
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
    async fn update_webhook_emits_audit_event_on_success() {
        let mut mock_audit_repo = MockResourceAuditLogRegistration::new();
        mock_audit_repo
            .expect_create()
            .times(1)
            .withf(move |event: &NewResourceAuditLogEvent| {
                event.resource_type == ResourceAuditResourceType::Webhook
                    && event.tenant_id.is_none()
                    && event.event == ResourceAuditEventKind::Updated
            })
            .returning(|_| Ok(()));

        let response = update_webhook(
            system_manager_profile(),
            Uuid::new_v4(),
            Some("new name".to_string()),
            None,
            None,
            test_config(),
            None,
            Box::new(&MockWebHookFetchingRepo),
            Box::new(&MockWebHookUpdatingRepo),
            Box::new(&MockEncryptionKeyFetchingRepo),
            Box::new(&mock_audit_repo),
        )
        .await
        .unwrap();

        assert!(matches!(response, UpdatingResponseKind::Updated(_)));
    }

    #[derive(Component)]
    #[shaku(interface = WebHookFetching)]
    struct MockWebHookNotFoundRepo;

    #[async_trait]
    impl WebHookFetching for MockWebHookNotFoundRepo {
        async fn get(
            &self,
            id: Uuid,
        ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::NotFound(Some(id)))
        }

        async fn list(
            &self,
            _: Option<String>,
            _: Option<WebHookTrigger>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<WebHook>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn list_by_trigger(
            &self,
            _: WebHookTrigger,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<WebHook>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn fetch_execution_event(
            &self,
            _: u32,
            _: u32,
            _: Option<
                Vec<crate::domain::dtos::webhook::WebHookExecutionStatus>,
            >,
        ) -> Result<
            mycelium_base::entities::FetchManyResponseKind<
                crate::domain::dtos::webhook::WebHookPayloadArtifact,
            >,
            MappedErrors,
        > {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn update_webhook_does_not_emit_audit_event_when_not_found() {
        let mut mock_audit_repo = MockResourceAuditLogRegistration::new();
        mock_audit_repo.expect_create().times(0);

        let response = update_webhook(
            system_manager_profile(),
            Uuid::new_v4(),
            Some("new name".to_string()),
            None,
            None,
            test_config(),
            None,
            Box::new(&MockWebHookNotFoundRepo),
            Box::new(&MockWebHookUpdatingRepo),
            Box::new(&MockEncryptionKeyFetchingRepo),
            Box::new(&mock_audit_repo),
        )
        .await;

        assert!(response.is_err());
    }
}
