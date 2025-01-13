use crate::{
    models::{
        config::DbConfig, owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
        tenant::Tenant as TenantModel,
    },
    schema::{
        owner_on_tenant as owner_on_tenant_model, tenant as tenant_model,
    },
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
    dtos::Children,
    entities::{CreateResponseKind, UpdatingResponseKind},
    utils::errors::{updating_err, MappedErrors},
};
use serde_json::{from_value, to_value, Value as JsonValue};
use shaku::Component;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantUpdating)]
pub struct TenantUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl TenantUpdating for TenantUpdatingSqlDbRepository {
    async fn update_name_and_description(
        &self,
        tenant_id: Uuid,
        tenant: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated = diesel::update(tenant_model::table.find(tenant_id))
            .set((
                tenant_model::name.eq(tenant.name),
                tenant_model::description.eq(tenant.description),
                tenant_model::updated.eq(Some(Local::now().naive_utc())),
            ))
            .get_result::<TenantModel>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update tenant: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(
            self.map_tenant_model_to_dto(updated),
        ))
    }

    async fn update_tenant_status(
        &self,
        tenant_id: Uuid,
        status: TenantStatus,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant = tenant_model::table
            .find(tenant_id)
            .select(tenant_model::status)
            .first::<Vec<JsonValue>>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let mut statuses: Vec<TenantStatus> = tenant
            .into_iter()
            .map(|s| serde_json::from_value(s).unwrap())
            .collect();

        statuses.push(status);

        let updated = diesel::update(tenant_model::table.find(tenant_id))
            .set((
                tenant_model::status.eq(statuses
                    .iter()
                    .map(|s| serde_json::to_value(s).unwrap())
                    .collect::<Vec<_>>()),
                tenant_model::updated.eq(Some(Local::now().naive_utc())),
            ))
            .get_result::<TenantModel>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update tenant: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(
            self.map_tenant_model_to_dto(updated),
        ))
    }

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

        // Check if relation already exists
        let exists = owner_on_tenant_model::table
            .filter(owner_on_tenant_model::tenant_id.eq(tenant_id))
            .filter(owner_on_tenant_model::owner_id.eq(owner_id))
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
            id: Uuid::new_v4(),
            tenant_id,
            owner_id,
            guest_by,
            created: Local::now().naive_utc(),
            updated: None,
        };

        let created = diesel::insert_into(owner_on_tenant_model::table)
            .values(&new_connection)
            .get_result::<OwnerOnTenantModel>(conn)
            .map_err(|e| {
                updating_err(format!(
                    "Failed to create owner connection: {}",
                    e
                ))
            })?;

        Ok(CreateResponseKind::Created(TenantOwnerConnection {
            tenant_id: created.tenant_id,
            owner_id: created.owner_id,
            guest_by: created.guest_by,
            created: created.created.and_local_timezone(Local).unwrap(),
            updated: created
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
        }))
    }

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

        // Get current tenant meta
        let tenant = tenant_model::table
            .find(tenant_id)
            .select(tenant_model::meta)
            .first::<Option<JsonValue>>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to fetch tenant meta: {}", e))
            })?;

        // Create or update meta map
        let mut meta_map: HashMap<String, String> =
            tenant.map(|m| from_value(m).unwrap()).unwrap_or_default();

        meta_map.insert(key.to_string(), value);

        // Update tenant meta
        diesel::update(tenant_model::table.find(tenant_id))
            .set((
                tenant_model::meta.eq(to_value(&meta_map).unwrap()),
                tenant_model::updated.eq(Some(Local::now().naive_utc())),
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

impl TenantUpdatingSqlDbRepository {
    fn map_tenant_model_to_dto(&self, model: TenantModel) -> Tenant {
        Tenant {
            id: Some(model.id),
            name: model.name,
            description: model.description,
            meta: model.meta.map(|m| serde_json::from_value(m).unwrap()),
            status: model
                .status
                .into_iter()
                .map(|s| serde_json::from_value(s).unwrap())
                .collect(),
            created: model.created.and_local_timezone(Local).unwrap(),
            updated: model
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
            owners: Children::Records(vec![]),
            manager: None,
            tags: None,
        }
    }
}
