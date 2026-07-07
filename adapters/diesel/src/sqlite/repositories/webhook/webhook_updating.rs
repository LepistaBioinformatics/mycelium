use super::map_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::{
        webhook::WebHook as WebHookModel,
        webhook_execution::WebHookExecution as WebHookExecutionModel,
    },
    schema::{webhook, webhook_execution},
    types::{json_to_text, naive_timestamp_to_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{prelude::*, result::DatabaseErrorKind, result::Error};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{WebHook, WebHookPayloadArtifact},
    },
    entities::WebHookUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = WebHookUpdating)]
pub struct WebHookUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl WebHookUpdating for WebHookUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_webhook", skip_all)]
    async fn update(
        &self,
        webhook_dto: WebHook,
    ) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let webhook_id = webhook_dto.id.ok_or_else(|| {
            updating_err("Unable to update webhook. Invalid record ID")
        })?;

        let secret_text =
            webhook_dto.to_owned().get_secret().as_ref().map(|s| {
                json_to_text(&serde_json::to_value(s).unwrap()).unwrap()
            });

        let updated =
            diesel::update(webhook::table.find(uuid_to_text(&webhook_id)))
                .set((
                    webhook::name.eq(webhook_dto.name.to_owned()),
                    webhook::description.eq(webhook_dto.description.to_owned()),
                    webhook::url.eq(webhook_dto.url.to_owned()),
                    webhook::trigger.eq(webhook_dto.trigger.to_string()),
                    webhook::is_active.eq(webhook_dto.is_active),
                    webhook::updated.eq(Some(naive_timestamp_to_text(
                        &Local::now().naive_utc(),
                    ))),
                    webhook::secret.eq(secret_text),
                ))
                .returning(WebHookModel::as_returning())
                .get_result::<WebHookModel>(conn)
                .map_err(|e| {
                    if e == diesel::result::Error::NotFound {
                        updating_err(format!(
                            "Invalid primary key: {:?}",
                            webhook_id
                        ))
                    } else {
                        updating_err(format!("Failed to update webhook: {}", e))
                    }
                })?;

        Ok(UpdatingResponseKind::Updated(map_model_to_dto(
            updated, true,
        )))
    }

    async fn update_execution_event(
        &self,
        artifact: WebHookPayloadArtifact,
    ) -> Result<UpdatingResponseKind<WebHookPayloadArtifact>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let artifact_id = match artifact.id {
            Some(id) => id,
            None => {
                return Err(updating_err(
                    "Unable to update webhook execution. Invalid record ID",
                )
                .with_code(NativeErrorCodes::MYC00001));
            }
        };

        let status = match artifact.status.to_owned() {
            Some(status) => status.to_string(),
            None => "unknown".to_string(),
        };

        let propagations_text = json_to_text(
            &serde_json::to_value(artifact.propagations.to_owned()).unwrap(),
        )
        .map_err(|e| {
            updating_err(format!("Failed to serialize propagations: {e}"))
        })?;

        diesel::update(
            webhook_execution::table.find(uuid_to_text(&artifact_id)),
        )
        .set((
            webhook_execution::attempts
                .eq(artifact.attempts.unwrap_or(0) as i32),
            webhook_execution::attempted
                .eq(Some(naive_timestamp_to_text(&Local::now().naive_utc()))),
            webhook_execution::status.eq(status),
            webhook_execution::propagations.eq(propagations_text),
        ))
        .returning(WebHookExecutionModel::as_returning())
        .get_result::<WebHookExecutionModel>(conn)
        .map_err(|e| match e {
            Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                updating_err("Webhook execution already exists".to_string())
                    .with_code(NativeErrorCodes::MYC00018)
                    .with_exp_true()
            }
            _ => {
                updating_err(format!("Failed to update webhook execution: {e}"))
            }
        })?;

        Ok(UpdatingResponseKind::Updated(artifact))
    }
}
