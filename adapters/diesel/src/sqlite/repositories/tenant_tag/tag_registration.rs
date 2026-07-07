use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::tenant_tag::TenantTag as TenantTagModel,
    schema::tenant_tag,
    types::{uuid_from_text, uuid_to_text},
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
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantTagRegistration)]
pub struct TenantTagRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TenantTagRegistration for TenantTagRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_tenant_tag", skip_all)]
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

        let meta_text = serde_json::to_string(&meta)
            .expect("meta map is always serializable");

        // Check if tag already exists
        let existing = tenant_tag::table
            .filter(tenant_tag::value.eq(&tag))
            .filter(tenant_tag::meta.eq(&meta_text))
            .filter(tenant_tag::tenant_id.eq(uuid_to_text(&tenant_id)))
            .select(TenantTagModel::as_select())
            .first::<TenantTagModel>(conn)
            .optional()
            .map_err(|e| creation_err(format!("Failed to check tag: {}", e)))?;

        if let Some(record) = existing {
            return Ok(GetOrCreateResponseKind::NotCreated(
                Tag {
                    id: uuid_from_text(&record.id).unwrap(),
                    value: record.value,
                    meta: record
                        .meta
                        .map(|m| serde_json::from_str(&m).unwrap()),
                },
                "Tag already exists".to_string(),
            ));
        }

        // Create new tag
        let new_tag = TenantTagModel {
            id: uuid_to_text(&Uuid::new_v4()),
            value: tag,
            meta: Some(meta_text),
            tenant_id: uuid_to_text(&tenant_id),
        };

        let created = diesel::insert_into(tenant_tag::table)
            .values(&new_tag)
            .returning(TenantTagModel::as_returning())
            .get_result::<TenantTagModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create tag: {}", e))
            })?;

        Ok(GetOrCreateResponseKind::Created(Tag {
            id: uuid_from_text(&created.id).unwrap(),
            value: created.value,
            meta: created.meta.map(|m| serde_json::from_str(&m).unwrap()),
        }))
    }
}
