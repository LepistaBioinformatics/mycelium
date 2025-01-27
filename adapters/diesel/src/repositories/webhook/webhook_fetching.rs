use crate::{
    models::{config::DbPoolProvider, webhook::WebHook as WebHookModel},
    schema::webhook as webhook_model,
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{WebHook, WebHookTrigger},
    },
    entities::WebHookFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookFetching)]
pub struct WebHookFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl WebHookFetching for WebHookFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_webhook", skip_all)]
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let webhook = webhook_model::table
            .find(id.to_string())
            .select(WebHookModel::as_select())
            .first::<WebHookModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhook: {}", e))
            })?;

        match webhook {
            Some(record) => {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.secret.map(|s| from_value(s).unwrap()),
                );

                webhook.id = Some(Uuid::from_str(&record.id).unwrap());
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());

                webhook.redact_secret_token();

                Ok(FetchResponseKind::Found(webhook))
            }
            None => Ok(FetchResponseKind::NotFound(Some(id))),
        }
    }

    #[tracing::instrument(name = "list_webhooks", skip_all)]
    async fn list(
        &self,
        name: Option<String>,
        trigger: Option<WebHookTrigger>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = webhook_model::table.into_boxed();

        if let Some(name) = name {
            query =
                query.filter(webhook_model::name.ilike(format!("%{}%", name)));
        }

        if let Some(trigger) = trigger {
            query =
                query.filter(webhook_model::trigger.eq(trigger.to_string()));
        }

        let webhooks = query
            .select(WebHookModel::as_select())
            .load::<WebHookModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhooks: {}", e))
            })?;

        if webhooks.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let webhooks = webhooks
            .into_iter()
            .map(|record| {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.secret.map(|s| from_value(s).unwrap()),
                );

                webhook.id = Some(Uuid::from_str(&record.id).unwrap());
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());

                webhook.redact_secret_token();
                webhook
            })
            .collect();

        Ok(FetchManyResponseKind::Found(webhooks))
    }

    #[tracing::instrument(name = "list_webhooks_by_trigger", skip_all)]
    async fn list_by_trigger(
        &self,
        trigger: WebHookTrigger,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let webhooks = webhook_model::table
            .filter(webhook_model::trigger.eq(trigger.to_string()))
            .filter(webhook_model::is_active.eq(true))
            .select(WebHookModel::as_select())
            .load::<WebHookModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhooks: {}", e))
            })?;

        if webhooks.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let webhooks = webhooks
            .into_iter()
            .map(|record| {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.secret.map(|s| from_value(s).unwrap()),
                );

                webhook.id = Some(Uuid::from_str(&record.id).unwrap());
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());

                webhook
            })
            .collect();

        Ok(FetchManyResponseKind::Found(webhooks))
    }
}
