use crate::{
    models::{config::DbPoolProvider, webhook::WebHook as WebHookModel},
    schema::webhook as webhook_model,
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, webhook::WebHook},
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
                webhook_model::name.eq(webhook.name),
                webhook_model::description.eq(webhook.description),
                webhook_model::url.eq(webhook.url),
                webhook_model::trigger.eq(webhook.trigger.to_string()),
                webhook_model::is_active.eq(webhook.is_active),
                webhook_model::updated.eq(Some(Local::now().naive_utc())),
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
}
