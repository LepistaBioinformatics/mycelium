use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::account_tag::AccountTag as AccountTagModel,
    schema::account_tag as account_tag_model,
    types::{json_from_text, json_to_text, uuid_from_text, uuid_to_text},
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{dtos::tag::Tag, entities::AccountTagRegistration};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTagRegistration)]
pub struct AccountTagRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl AccountTagRegistration for AccountTagRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_account_tag", skip_all)]
    async fn get_or_create(
        &self,
        account_id: Uuid,
        tag: String,
        meta: HashMap<String, String>,
    ) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
        })?;

        let meta_text = json_to_text(&serde_json::to_value(&meta).unwrap())
            .map_err(|e| {
                creation_err(format!("Failed to serialize tag meta: {e}"))
            })?;

        // Check if tag already exists
        let existing_tag = account_tag_model::table
            .filter(account_tag_model::value.eq(&tag))
            .filter(account_tag_model::meta.eq(&meta_text))
            .filter(account_tag_model::account_id.eq(uuid_to_text(&account_id)))
            .select(AccountTagModel::as_select())
            .first::<AccountTagModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing tag: {}", e))
            })?;

        if let Some(record) = existing_tag {
            return Ok(GetOrCreateResponseKind::NotCreated(
                Tag {
                    id: uuid_from_text(&record.id).unwrap(),
                    value: record.value,
                    meta: record.meta.map(|m| {
                        serde_json::from_value(json_from_text(&m).unwrap())
                            .unwrap()
                    }),
                },
                "Tag already exists".to_string(),
            ));
        }

        // Create new tag
        let new_tag = AccountTagModel {
            id: uuid_to_text(&Uuid::new_v4()),
            value: tag,
            meta: Some(meta_text),
            account_id: uuid_to_text(&account_id),
        };

        let created_tag = diesel::insert_into(account_tag_model::table)
            .values(&new_tag)
            .returning(AccountTagModel::as_returning())
            .get_result::<AccountTagModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create tag: {}", e))
            })?;

        Ok(GetOrCreateResponseKind::Created(Tag {
            id: uuid_from_text(&created_tag.id).unwrap(),
            value: created_tag.value,
            meta: created_tag.meta.map(|m| {
                serde_json::from_value(json_from_text(&m).unwrap()).unwrap()
            }),
        }))
    }
}
