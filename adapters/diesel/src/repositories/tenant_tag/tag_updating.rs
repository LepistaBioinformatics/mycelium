use crate::{
    models::{config::DbPoolProvider, tenant_tag::TenantTag as TenantTagModel},
    schema::tenant_tag as tenant_tag_model,
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
use serde_json::{from_value, to_value};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = TenantTagUpdating)]
pub struct TenantTagUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TenantTagUpdating for TenantTagUpdatingSqlDbRepository {
    async fn update(
        &self,
        tag: Tag,
    ) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated = diesel::update(tenant_tag_model::table.find(tag.id))
            .set((
                tenant_tag_model::value.eq(tag.value),
                tenant_tag_model::meta.eq(Some(to_value(&tag.meta).unwrap())),
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
            id: updated.id,
            value: updated.value,
            meta: updated.meta.map(|m| from_value(m).unwrap()),
        }))
    }
}
