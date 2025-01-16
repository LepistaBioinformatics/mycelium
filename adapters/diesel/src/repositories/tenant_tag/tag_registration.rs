use crate::{
    models::{config::DbPoolProvider, tenant_tag::TenantTag as TenantTagModel},
    schema::tenant_tag as tenant_tag_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, tag::Tag},
    entities::TenantTagRegistration,
};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantTagRegistration)]
pub struct TenantTagRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TenantTagRegistration for TenantTagRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        tenant_id: Uuid,
        tag: String,
        meta: HashMap<String, String>,
    ) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if tag already exists
        let existing = tenant_tag_model::table
            .filter(tenant_tag_model::value.eq(&tag))
            .filter(tenant_tag_model::tenant_id.eq(tenant_id))
            .select(TenantTagModel::as_select())
            .first::<TenantTagModel>(conn)
            .optional()
            .map_err(|e| creation_err(format!("Failed to check tag: {}", e)))?;

        if let Some(record) = existing {
            return Ok(GetOrCreateResponseKind::NotCreated(
                Tag {
                    id: record.id,
                    value: record.value,
                    meta: record.meta.map(|m| from_value(m).unwrap()),
                },
                "Tag already exists".to_string(),
            ));
        }

        // Create new tag
        let new_tag = TenantTagModel {
            id: Uuid::new_v4(),
            value: tag,
            meta: Some(to_value(&meta).unwrap()),
            tenant_id,
        };

        let created = diesel::insert_into(tenant_tag_model::table)
            .values(&new_tag)
            .returning(TenantTagModel::as_returning())
            .get_result::<TenantTagModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create tag: {}", e))
            })?;

        Ok(GetOrCreateResponseKind::Created(Tag {
            id: created.id,
            value: created.value,
            meta: created.meta.map(|m| from_value(m).unwrap()),
        }))
    }
}
