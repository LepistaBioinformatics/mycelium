use crate::{
    models::{
        config::DbPoolProvider,
        owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
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
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TenantUpdating for TenantUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_name_and_description", skip_all)]
    async fn update_name_and_description(
        &self,
        tenant_id: Uuid,
        tenant: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated =
            diesel::update(tenant_model::table.find(tenant_id.to_string()))
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

        let tenant = tenant_model::table
            .find(tenant_id.to_string())
            .select(tenant_model::status)
            .first::<Option<Vec<JsonValue>>>(conn)
            .map_err(|e| {
                updating_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let mut statuses: Vec<TenantStatus> = tenant
            .unwrap_or_default()
            .into_iter()
            .map(|s| serde_json::from_value(s).unwrap())
            .collect();

        statuses.push(status);

        let updated =
            diesel::update(tenant_model::table.find(tenant_id.to_string()))
                .set((
                    tenant_model::status.eq(Some(
                        statuses
                            .iter()
                            .map(|s| serde_json::to_value(s).unwrap())
                            .collect::<Vec<_>>(),
                    )),
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

        // Check if relation already exists
        let exists = owner_on_tenant_model::table
            .filter(owner_on_tenant_model::tenant_id.eq(tenant_id.to_string()))
            .filter(owner_on_tenant_model::owner_id.eq(owner_id.to_string()))
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
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            owner_id: owner_id.to_string(),
            guest_by,
            created: Local::now().naive_utc(),
            updated: None,
        };

        let created = diesel::insert_into(owner_on_tenant_model::table)
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
            tenant_id: Uuid::from_str(&created.tenant_id).unwrap(),
            owner_id: Uuid::from_str(&created.owner_id).unwrap(),
            guest_by: created.guest_by,
            created: created.created.and_local_timezone(Local).unwrap(),
            updated: created
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
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

        // Get current tenant meta
        let tenant = tenant_model::table
            .find(tenant_id.to_string())
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
        diesel::update(tenant_model::table.find(tenant_id.to_string()))
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
            id: Some(Uuid::from_str(&model.id).unwrap()),
            name: model.name,
            description: model.description,
            meta: model.meta.map(|m| {
                serde_json::from_value::<HashMap<String, String>>(m)
                    .unwrap()
                    .iter()
                    .map(|(k, v)| {
                        (TenantMetaKey::from_str(k).unwrap(), v.to_string())
                    })
                    .collect()
            }),
            status: model
                .status
                .unwrap_or_default()
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
