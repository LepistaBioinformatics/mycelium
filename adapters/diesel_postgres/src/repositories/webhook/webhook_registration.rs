use crate::{
    models::{config::DbPoolProvider, webhook::WebHook as WebHookModel},
    repositories::parse_optional_written_by,
    schema::{
        webhook as webhook_model, webhook_execution as webhook_execution_model,
    },
};

use crate::models::webhook_execution::WebHookExecution as WebHookExecutionModel;
use async_trait::async_trait;
use chrono::Local;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error},
};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{WebHook, WebHookPayloadArtifact},
    },
    entities::WebHookRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookRegistration)]
pub struct WebHookRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl WebHookRegistration for WebHookRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_webhook", skip_all)]
    async fn create(
        &self,
        webhook: WebHook,
    ) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let new_webhook = WebHookModel {
            id: Uuid::new_v4(),
            name: webhook.name.clone(),
            description: webhook.description.clone(),
            url: webhook.url.clone(),
            trigger: webhook.trigger.to_string(),
            method: webhook.method.map(|m| m.to_string()),
            secret: webhook.get_secret().map(|s| to_value(s).unwrap()),
            is_active: webhook.is_active,
            created: Local::now().naive_utc(),
            created_by: webhook.created_by.map(|m| to_value(m).unwrap()),
            updated: None,
            updated_by: None,
        };

        let created = diesel::insert_into(webhook_model::table)
            .values(&new_webhook)
            .returning(WebHookModel::as_returning())
            .get_result::<WebHookModel>(conn)
            .map_err(|e| match e {
                Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                    creation_err("Webhook already exists".to_string())
                        .with_code(NativeErrorCodes::MYC00018)
                        .with_exp_true()
                }
                _ => creation_err(format!("Failed to create webhook: {}", e)),
            })?;

        let mut webhook = WebHook::new(
            created.name,
            created.description,
            created.url,
            created.trigger.parse().unwrap(),
            created.method.map(|m| m.parse().unwrap()),
            created.secret.map(|s| from_value(s).unwrap()),
            parse_optional_written_by(created.created_by),
        );

        webhook.id = Some(created.id);
        webhook.is_active = created.is_active;
        webhook.created = created.created.and_local_timezone(Local).unwrap();
        webhook.updated = created
            .updated
            .map(|dt| dt.and_local_timezone(Local).unwrap());
        webhook.updated_by = parse_optional_written_by(created.updated_by);

        webhook.redact_secret_token();

        Ok(CreateResponseKind::Created(webhook))
    }

    #[tracing::instrument(name = "register_execution_event", skip_all)]
    async fn register_execution_event(
        &self,
        artifact: WebHookPayloadArtifact,
    ) -> Result<CreateResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let new_webhook_execution = WebHookExecutionModel {
            id: artifact.id.unwrap_or(Uuid::new_v4()),
            payload: artifact.payload,
            payload_id: artifact.payload_id.to_string(),
            trigger: artifact.trigger.to_string(),
            created: Local::now().naive_utc(),
            status: match artifact.status {
                Some(status) => Some(status.to_string()),
                None => None,
            },
            attempts: 0,
            attempted: None,
            propagations: None,
            encrypted: None,
        };

        let created = diesel::insert_into(webhook_execution_model::table)
            .values(&new_webhook_execution)
            .returning(WebHookExecutionModel::as_returning())
            .get_result::<WebHookExecutionModel>(conn)
            .map_err(|e| match e {
                Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                    creation_err("Webhook already exists".to_string())
                        .with_code(NativeErrorCodes::MYC00018)
                        .with_exp_true()
                }
                _ => {
                    tracing::error!("Failed to create webhook execution: {e}");
                    creation_err("Failed to create webhook execution")
                }
            })?;

        Ok(CreateResponseKind::Created(created.id))
    }
}
