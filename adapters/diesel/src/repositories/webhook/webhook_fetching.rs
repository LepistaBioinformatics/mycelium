use crate::{
    models::{
        config::DbPoolProvider, webhook::WebHook as WebHookModel,
        webhook_execution::WebHookExecution as WebHookExecutionModel,
    },
    repositories::parse_optional_written_by,
    schema::{
        webhook as webhook_model, webhook_execution as webhook_execution_model,
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{
            PayloadId, WebHook, WebHookExecutionStatus, WebHookPayloadArtifact,
            WebHookTrigger,
        },
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
            .find(id)
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
                    parse_optional_written_by(record.created_by),
                );

                webhook.id = Some(record.id);
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());
                webhook.updated_by =
                    parse_optional_written_by(record.updated_by);

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
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let base_query = webhook_model::table;
        let mut count_query = base_query.into_boxed();
        let mut records_query = base_query.into_boxed();

        if let Some(name) = name {
            let dsl = webhook_model::name.ilike(format!("%{}%", name));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(trigger) = trigger {
            let dsl = webhook_model::trigger.eq(trigger.to_string());
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let records = records_query
            .select(WebHookModel::as_select())
            .order_by(webhook_model::created.desc())
            .limit(page_size)
            .offset(skip)
            .load::<WebHookModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhooks: {}", e))
            })?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let total = count_query
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to count webhooks: {}", e))
            })?;

        let webhooks = records
            .into_iter()
            .map(|record| {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.secret.map(|s| from_value(s).unwrap()),
                    parse_optional_written_by(record.created_by),
                );

                webhook.id = Some(record.id);
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());
                webhook.updated_by =
                    parse_optional_written_by(record.updated_by);

                webhook.redact_secret_token();
                webhook
            })
            .collect();

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip),
            size: Some(page_size),
            records: webhooks,
        })
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
                    parse_optional_written_by(record.created_by),
                );

                webhook.id = Some(record.id);
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());
                webhook.updated_by =
                    parse_optional_written_by(record.updated_by);

                webhook
            })
            .collect();

        Ok(FetchManyResponseKind::Found(webhooks))
    }

    #[tracing::instrument(name = "fetch_execution_event", skip_all)]
    async fn fetch_execution_event(
        &self,
        max_events: u32,
        max_attempts: u32,
        status: Option<Vec<WebHookExecutionStatus>>,
    ) -> Result<FetchManyResponseKind<WebHookPayloadArtifact>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let statuses = status
            .unwrap_or(vec![WebHookExecutionStatus::Pending])
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let execution_events =
            webhook_execution_model::table
                .filter(webhook_execution_model::status.eq_any(statuses).and(
                    webhook_execution_model::attempts.lt(max_attempts as i32),
                ))
                .order(webhook_execution_model::created.desc())
                .limit(max_events as i64)
                .select(WebHookExecutionModel::as_select())
                .load::<WebHookExecutionModel>(conn)
                .map_err(|e| {
                    fetching_err(format!(
                        "Failed to fetch webhook execution events: {e}"
                    ))
                })?;

        let execution_events = execution_events
            .into_iter()
            .map(|record| WebHookPayloadArtifact {
                id: Some(record.id),
                payload: record.payload.to_string(),
                payload_id: PayloadId::from_str(&record.payload_id).unwrap(),
                trigger: record.trigger.parse().unwrap(),
                propagations: match record.propagations {
                    Some(propagations) => {
                        Some(from_value(propagations).unwrap())
                    }
                    None => None,
                },
                encrypted: record.encrypted,
                attempts: Some(record.attempts as u8),
                attempted: record
                    .attempted
                    .map(|a| a.and_local_timezone(Local).unwrap()),
                created: Some(
                    record.created.and_local_timezone(Local).unwrap(),
                ),
                status: record
                    .status
                    .map(|s| WebHookExecutionStatus::from_str(&s).unwrap()),
            })
            .collect();

        Ok(FetchManyResponseKind::Found(execution_events))
    }
}
