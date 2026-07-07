use crate::{
    models::{
        account_tag::AccountTag as AccountTagModel, config::DbPoolProvider,
    },
    schema::account_tag as account_tag_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{dtos::tag::Tag, entities::AccountTagUpdating};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use serde_json::to_value;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = AccountTagUpdating)]
pub struct AccountTagUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
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

        let tag_id = tag.id;

        let updated_tag = diesel::update(account_tag_model::table.find(tag_id))
            .set((
                account_tag_model::value.eq(tag.value),
                account_tag_model::meta.eq(Some(to_value(&tag.meta).unwrap())),
            ))
            .get_result::<AccountTagModel>(conn)
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    updating_err(format!("Invalid primary key: {:?}", tag_id))
                } else {
                    updating_err(format!("Failed to update tag: {}", e))
                }
            })?;

        Ok(UpdatingResponseKind::Updated(Tag {
            id: updated_tag.id,
            value: updated_tag.value,
            meta: updated_tag.meta.map(|m| serde_json::from_value(m).unwrap()),
        }))
    }
}
