use super::map_model_to_dto;
use crate::{
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
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookRegistration)]
pub struct WebHookRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl WebHookRegistration for WebHookRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_webhook", skip_all)]
    async fn create(
        &self,
        webhook_dto: WebHook,
    ) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let new_webhook = WebHookModel {
            id: uuid_to_text(&Uuid::new_v4()),
            name: webhook_dto.name.clone(),
            description: webhook_dto.description.clone(),
            url: webhook_dto.url.clone(),
            trigger: webhook_dto.trigger.to_string(),
            method: webhook_dto.method.map(|m| m.to_string()),
            secret: webhook_dto.get_secret().map(|s| {
                json_to_text(&serde_json::to_value(s).unwrap()).unwrap()
            }),
            is_active: webhook_dto.is_active,
            created: naive_timestamp_to_text(&Local::now().naive_utc()),
            created_by: webhook_dto
                .created_by
                .map(|m| serde_json::to_string(&m).unwrap()),
            updated: None,
            updated_by: None,
        };

        let created = diesel::insert_into(webhook::table)
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

        Ok(CreateResponseKind::Created(map_model_to_dto(created, true)))
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
            id: uuid_to_text(&artifact.id.unwrap_or(Uuid::new_v4())),
            payload: artifact.payload,
            payload_id: artifact.payload_id.to_string(),
            trigger: artifact.trigger.to_string(),
            created: naive_timestamp_to_text(&Local::now().naive_utc()),
            status: artifact.status.map(|status| status.to_string()),
            attempts: 0,
            attempted: None,
            propagations: None,
            encrypted: None,
        };

        let created = diesel::insert_into(webhook_execution::table)
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

        Ok(CreateResponseKind::Created(
            crate::types::uuid_from_text(&created.id).unwrap(),
        ))
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::webhook::{
            WebHookDeletionSqlDbRepository, WebHookFetchingSqlDbRepository,
            WebHookUpdatingSqlDbRepository,
        },
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::webhook::{
            PayloadId, WebHook, WebHookExecutionStatus, WebHookPayloadArtifact,
            WebHookTrigger,
        },
        entities::{WebHookDeletion, WebHookFetching, WebHookUpdating},
    };
    use mycelium_base::entities::{
        DeletionResponseKind, FetchManyResponseKind, FetchResponseKind,
        UpdatingResponseKind,
    };

    #[tokio::test]
    async fn webhook_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = WebHookRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = WebHookFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = WebHookUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = WebHookDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create
        let webhook_dto = WebHook::new(
            "Acme Hook".into(),
            None,
            "https://acme.test/webhook".into(),
            WebHookTrigger::SubscriptionAccountCreated,
            None,
            None,
            None,
        );
        let created = match registration.create(webhook_dto).await? {
            CreateResponseKind::Created(hook) => hook,
            CreateResponseKind::NotCreated(..) => {
                panic!("expected the webhook to be created")
            }
        };
        let webhook_id = created.id.expect("created webhook must have an id");

        // Fetch
        let found = match fetching.get(webhook_id).await? {
            FetchResponseKind::Found(hook) => hook,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the webhook to be found")
            }
        };
        assert_eq!(found.name, "Acme Hook");

        // Update
        let mut update_payload = found.clone();
        update_payload.name = "Acme Hook Updated".into();
        let updated = match updating.update(update_payload).await? {
            UpdatingResponseKind::Updated(hook) => hook,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the webhook to be updated")
            }
        };
        assert_eq!(updated.name, "Acme Hook Updated");

        // list_by_trigger finds the active hook
        let by_trigger = match fetching
            .list_by_trigger(WebHookTrigger::SubscriptionAccountCreated)
            .await?
        {
            FetchManyResponseKind::Found(hooks) => hooks,
            _ => panic!("expected to find webhooks by trigger"),
        };
        assert_eq!(by_trigger.len(), 1);

        // Register + fetch + update an execution event
        let artifact = WebHookPayloadArtifact::new(
            None,
            "{\"event\":\"created\"}".into(),
            PayloadId::Uuid(Uuid::new_v4()),
            WebHookTrigger::SubscriptionAccountCreated,
        );
        let execution_id =
            match registration.register_execution_event(artifact).await? {
                CreateResponseKind::Created(id) => id,
                CreateResponseKind::NotCreated(..) => {
                    panic!("expected the execution event to be created")
                }
            };

        let pending = match fetching
            .fetch_execution_event(
                10,
                5,
                Some(vec![WebHookExecutionStatus::Pending]),
            )
            .await?
        {
            FetchManyResponseKind::Found(events) => events,
            _ => panic!("expected to find pending execution events"),
        };
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, Some(execution_id));

        let mut executed = pending[0].clone();
        executed.status = Some(WebHookExecutionStatus::Success);
        executed.attempts = Some(1);
        updating.update_execution_event(executed).await?;

        let still_pending = fetching
            .fetch_execution_event(
                10,
                5,
                Some(vec![WebHookExecutionStatus::Pending]),
            )
            .await?;
        assert!(matches!(
            still_pending,
            FetchManyResponseKind::Found(ref v) if v.is_empty()
        ));

        // Delete
        let deleted = deletion.delete(webhook_id).await?;
        assert!(matches!(deleted, DeletionResponseKind::Deleted));

        Ok(())
    }
}
