use super::{decode_status, map_tenant_model_to_dto};
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::{
        owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
        tenant::Tenant as TenantModel,
    },
    repositories::account::created_at_from_text,
    schema::{owner_on_tenant, tenant},
    types::{json_array_to_text, naive_timestamp_to_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        tenant::{Tenant, TenantMeta, TenantMetaKey, TenantStatus},
    },
    entities::{TenantOwnerConnection, TenantUpdating},
};
use mycelium_base::{
    entities::{CreateResponseKind, UpdatingResponseKind},
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantUpdating)]
pub struct TenantUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TenantUpdating for TenantUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_name_and_description", skip_all)]
    async fn update_name_and_description(
        &self,
        tenant_id: Uuid,
        tenant_dto: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated =
            diesel::update(tenant::table.find(uuid_to_text(&tenant_id)))
                .set((
                    tenant::name.eq(tenant_dto.name),
                    tenant::description.eq(tenant_dto.description),
                    tenant::updated.eq(Some(naive_timestamp_to_text(
                        &Local::now().naive_utc(),
                    ))),
                ))
                .returning(TenantModel::as_returning())
                .get_result::<TenantModel>(conn)
                .map_err(|e| {
                    updating_err(format!("Failed to update tenant: {}", e))
                })?;

        Ok(UpdatingResponseKind::Updated(map_tenant_model_to_dto(
            updated,
        )))
    }

    #[tracing::instrument(name = "update_tenant_status", skip_all)]
    async fn update_tenant_status(
        &self,
        tenant_id: Uuid,
        status: TenantStatus,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);

        let current_status = tenant::table
            .find(&tenant_id_text)
            .select(tenant::status)
            .first::<Option<String>>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let mut statuses = decode_status(current_status);
        statuses.push(status);

        let status_json = statuses
            .iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect::<Vec<_>>();

        let status_text = json_array_to_text(&status_json).map_err(|e| {
            updating_err(format!("Failed to serialize status: {e}"))
        })?;

        let updated = diesel::update(tenant::table.find(&tenant_id_text))
            .set((
                tenant::status.eq(Some(status_text)),
                tenant::updated.eq(Some(naive_timestamp_to_text(
                    &Local::now().naive_utc(),
                ))),
            ))
            .returning(TenantModel::as_returning())
            .get_result::<TenantModel>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update tenant: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(map_tenant_model_to_dto(
            updated,
        )))
    }

    #[tracing::instrument(name = "register_owner", skip_all)]
    async fn register_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
        guest_by: String,
    ) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);
        let owner_id_text = uuid_to_text(&owner_id);

        // Check if relation already exists
        let exists = owner_on_tenant::table
            .filter(owner_on_tenant::tenant_id.eq(&tenant_id_text))
            .filter(owner_on_tenant::owner_id.eq(&owner_id_text))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| {
                updating_err(format!(
                    "Failed to check existing relation: {}",
                    e
                ))
            })?;

        if exists > 0 {
            return Ok(CreateResponseKind::NotCreated(
                TenantOwnerConnection {
                    tenant_id,
                    owner_id,
                    guest_by,
                    created: Local::now(),
                    updated: None,
                },
                "Owner is already registered on this tenant".to_string(),
            ));
        }

        // Create new relation
        let new_connection = OwnerOnTenantModel {
            id: uuid_to_text(&Uuid::new_v4()),
            tenant_id: tenant_id_text,
            owner_id: owner_id_text,
            guest_by,
            created: naive_timestamp_to_text(&Local::now().naive_utc()),
            updated: None,
        };

        let created = diesel::insert_into(owner_on_tenant::table)
            .values(&new_connection)
            .returning(OwnerOnTenantModel::as_returning())
            .get_result(conn)
            .map_err(|e| {
                updating_err(format!(
                    "Failed to create owner connection: {}",
                    e
                ))
            })?;

        Ok(CreateResponseKind::Created(TenantOwnerConnection {
            tenant_id,
            owner_id,
            guest_by: created.guest_by,
            created: created_at_from_text(&created.created),
            updated: created.updated.map(|dt| created_at_from_text(&dt)),
        }))
    }

    #[tracing::instrument(name = "update_tenant_meta", skip_all)]
    async fn update_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<UpdatingResponseKind<TenantMeta>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);

        // Get current tenant meta
        let meta = tenant::table
            .find(&tenant_id_text)
            .select(tenant::meta)
            .first::<Option<String>>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to fetch tenant meta: {}", e))
            })?;

        // Create or update meta map
        let mut meta_map: std::collections::HashMap<String, String> = meta
            .map(|m| serde_json::from_str(&m).unwrap())
            .unwrap_or_default();

        meta_map.insert(key.to_string(), value);

        let meta_text = serde_json::to_string(&meta_map)
            .expect("meta map is always serializable");

        // Update tenant meta
        diesel::update(tenant::table.find(&tenant_id_text))
            .set((
                tenant::meta.eq(meta_text),
                tenant::updated.eq(Some(naive_timestamp_to_text(
                    &Local::now().naive_utc(),
                ))),
            ))
            .execute(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update tenant meta: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(
            meta_map
                .into_iter()
                .map(|(k, v)| (TenantMetaKey::from_str(&k).unwrap(), v))
                .collect(),
        ))
    }
}
