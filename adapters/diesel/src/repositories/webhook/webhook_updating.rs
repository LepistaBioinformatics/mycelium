use crate::{
    models::{
        config::DbPoolProvider, webhook::WebHook as WebHookModel,
        webhook_execution::WebHookExecution as WebHookExecutionModel,
    },
    schema::webhook as webhook_model,
    schema::webhook_execution as webhook_execution_model,
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
use serde_json::from_value;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = WebHookUpdating)]
pub struct WebHookUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl WebHookUpdating for WebHookUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_webhook", skip_all)]
    async fn update(
        &self,
        webhook: WebHook,
    ) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let webhook_id = webhook.id.ok_or_else(|| {
            updating_err("Unable to update webhook. Invalid record ID")
        })?;

        let updated = diesel::update(webhook_model::table.find(webhook_id))
            .set((
                webhook_model::name.eq(webhook.name.to_owned()),
                webhook_model::description.eq(webhook.description.to_owned()),
                webhook_model::url.eq(webhook.url.to_owned()),
                webhook_model::trigger.eq(webhook.trigger.to_string()),
                webhook_model::is_active.eq(webhook.is_active),
                webhook_model::updated.eq(Some(Local::now().naive_utc())),
                webhook_model::secret.eq(webhook
                    .to_owned()
                    .get_secret()
                    .as_ref()
                    .map(|s| serde_json::to_value(s).unwrap())),
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

        let mut webhook = WebHook::new(
            updated.name,
            updated.description,
            updated.url,
            updated.trigger.parse().unwrap(),
            updated.secret.map(|s| from_value(s).unwrap()),
        );

        webhook.id = Some(updated.id);
        webhook.is_active = updated.is_active;
        webhook.created = updated.created.and_local_timezone(Local).unwrap();
        webhook.updated = updated
            .updated
            .map(|dt| dt.and_local_timezone(Local).unwrap());

        webhook.redact_secret_token();

        Ok(UpdatingResponseKind::Updated(webhook))
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

        diesel::update(webhook_execution_model::table.find(artifact_id))
            .set((
                webhook_execution_model::attempts
                    .eq(artifact.attempts.unwrap_or(0) as i32),
                webhook_execution_model::attempted
                    .eq(Some(Local::now().naive_utc())),
                webhook_execution_model::status.eq(status),
                webhook_execution_model::propagations
                    .eq(serde_json::to_value(artifact.propagations.to_owned())
                        .unwrap()),
            ))
            .returning(WebHookExecutionModel::as_returning())
            .get_result::<WebHookExecutionModel>(conn)
            .map_err(|e| match e {
                Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                    updating_err("Webhook execution already exists".to_string())
                        .with_code(NativeErrorCodes::MYC00018)
                        .with_exp_true()
                }
                _ => updating_err(format!(
                    "Failed to update webhook execution: {e}"
                )),
            })?;

        Ok(UpdatingResponseKind::Updated(artifact))
    }
}
