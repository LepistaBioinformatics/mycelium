use crate::{
    models::{
        account_tag::AccountTag as AccountTagModel, config::DbPoolProvider,
    },
    schema::account_tag as account_tag_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{dtos::tag::Tag, entities::AccountTagRegistration};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::to_value;
use shaku::Component;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTagRegistration)]
pub struct AccountTagRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl AccountTagRegistration for AccountTagRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_account_tag", skip_all)]
    async fn get_or_create(
        &self,
        analysis_id: Uuid,
        tag: String,
        meta: HashMap<String, String>,
    ) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
        })?;

        // Check if tag already exists
        let existing_tag = account_tag_model::table
            .filter(account_tag_model::value.eq(&tag))
            .filter(account_tag_model::account_id.eq(analysis_id.to_string()))
            .select(AccountTagModel::as_select())
            .first::<AccountTagModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing tag: {}", e))
            })?;

        if let Some(record) = existing_tag {
            return Ok(GetOrCreateResponseKind::NotCreated(
                Tag {
                    id: Uuid::from_str(&record.id).unwrap(),
                    value: record.value,
                    meta: record
                        .meta
                        .map(|m| serde_json::from_value(m).unwrap()),
                },
                "Tag already exists".to_string(),
            ));
        }

        // Create new tag
        let new_tag = AccountTagModel {
            id: Uuid::new_v4().to_string(),
            value: tag,
            meta: Some(to_value(&meta).unwrap()),
            account_id: analysis_id.to_string(),
        };

        let created_tag = diesel::insert_into(account_tag_model::table)
            .values(&new_tag)
            .get_result::<AccountTagModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create tag: {}", e))
            })?;

        Ok(GetOrCreateResponseKind::Created(Tag {
            id: Uuid::from_str(&created_tag.id).unwrap(),
            value: created_tag.value,
            meta: created_tag.meta.map(|m| serde_json::from_value(m).unwrap()),
        }))
    }
}
