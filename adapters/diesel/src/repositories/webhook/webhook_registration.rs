use crate::{
    models::{config::DbPoolProvider, webhook::WebHook as WebHookModel},
    schema::webhook as webhook_model,
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error},
};
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, webhook::WebHook},
    entities::WebHookRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{str::FromStr, sync::Arc};
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
            id: Uuid::new_v4().to_string(),
            name: webhook.name.clone(),
            description: webhook.description.clone(),
            url: webhook.url.clone(),
            trigger: webhook.trigger.to_string(),
            secret: webhook.get_secret().map(|s| to_value(s).unwrap()),
            is_active: webhook.is_active,
            created: Local::now().naive_utc(),
            updated: None,
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
            created.secret.map(|s| from_value(s).unwrap()),
        );

        webhook.id = Some(Uuid::from_str(&created.id).unwrap());
        webhook.is_active = created.is_active;
        webhook.created = created.created.and_local_timezone(Local).unwrap();
        webhook.updated = created
            .updated
            .map(|dt| dt.and_local_timezone(Local).unwrap());

        webhook.redact_secret_token();

        Ok(CreateResponseKind::Created(webhook))
    }
}
