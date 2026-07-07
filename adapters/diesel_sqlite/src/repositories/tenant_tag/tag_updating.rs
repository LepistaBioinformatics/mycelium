use crate::{
    config::SqliteDbPoolProvider,
    models::tenant_tag::TenantTag as TenantTagModel,
    schema::tenant_tag,
    types::{uuid_from_text, uuid_to_text},
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, tag::Tag},
    entities::TenantTagUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = TenantTagUpdating)]
pub struct TenantTagUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TenantTagUpdating for TenantTagUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_tenant_tag", skip_all)]
    async fn update(
        &self,
        tag: Tag,
    ) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tag_id = uuid_to_text(&tag.id);

        let meta_text = serde_json::to_string(&tag.meta)
            .expect("meta map is always serializable");

        let updated = diesel::update(tenant_tag::table.find(tag_id))
            .set((
                tenant_tag::value.eq(tag.value),
                tenant_tag::meta.eq(Some(meta_text)),
            ))
            .returning(TenantTagModel::as_returning())
            .get_result::<TenantTagModel>(conn)
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    updating_err(format!("Invalid primary key: {:?}", tag.id))
                } else {
                    updating_err(format!("Failed to update tag: {}", e))
                }
            })?;

        Ok(UpdatingResponseKind::Updated(Tag {
            id: uuid_from_text(&updated.id).unwrap(),
            value: updated.value,
            meta: updated.meta.map(|m| serde_json::from_str(&m).unwrap()),
        }))
    }
}
