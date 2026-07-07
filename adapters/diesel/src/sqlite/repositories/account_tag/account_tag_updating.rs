use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::account_tag::AccountTag as AccountTagModel,
    schema::account_tag as account_tag_model,
    types::{json_from_text, json_to_text, uuid_from_text, uuid_to_text},
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{dtos::tag::Tag, entities::AccountTagUpdating};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = AccountTagUpdating)]
pub struct AccountTagUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl AccountTagUpdating for AccountTagUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_account_tag", skip_all)]
    async fn update(
        &self,
        tag: Tag,
    ) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
        })?;

        let tag_id = uuid_to_text(&tag.id);

        let meta_text = json_to_text(&serde_json::to_value(&tag.meta).unwrap())
            .map_err(|e| {
                updating_err(format!("Failed to serialize tag meta: {e}"))
            })?;

        let updated_tag = diesel::update(account_tag_model::table.find(tag_id))
            .set((
                account_tag_model::value.eq(tag.value),
                account_tag_model::meta.eq(Some(meta_text)),
            ))
            .returning(AccountTagModel::as_returning())
            .get_result::<AccountTagModel>(conn)
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    updating_err(format!("Invalid primary key: {:?}", tag.id))
                } else {
                    updating_err(format!("Failed to update tag: {}", e))
                }
            })?;

        Ok(UpdatingResponseKind::Updated(Tag {
            id: uuid_from_text(&updated_tag.id).unwrap(),
            value: updated_tag.value,
            meta: updated_tag.meta.map(|m| {
                serde_json::from_value(json_from_text(&m).unwrap()).unwrap()
            }),
        }))
    }
}
