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
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantRegistration,
};
use mycelium_base::{
    dtos::Children,
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::to_value;
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantRegistration)]
pub struct TenantRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TenantRegistration for TenantRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_tenant", skip_all)]
    async fn create(
        &self,
        tenant: Tenant,
        guest_by: String,
    ) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Verificar se já existe um tenant com o mesmo nome
        let existing = tenant_model::table
            .filter(tenant_model::name.eq(&tenant.name))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing tenant: {}", e))
            })?;

        if let Some(record) = existing {
            return Ok(CreateResponseKind::NotCreated(
                Tenant {
                    id: Some(record.id),
                    name: record.name,
                    description: record.description,
                    meta: record
                        .meta
                        .map(|m| serde_json::from_value(m).unwrap()),
                    status: record
                        .status
                        .unwrap_or_default()
                        .into_iter()
                        .map(|s| serde_json::from_value(s).unwrap())
                        .collect(),
                    created: record.created.and_local_timezone(Local).unwrap(),
                    updated: record
                        .updated
                        .map(|dt| dt.and_local_timezone(Local).unwrap()),
                    owners: Children::Records(vec![]),
                    manager: None,
                    tags: None,
                },
                "Tenant already exists".to_string(),
            ));
        }

        // Criar novo tenant
        let tenant_id = Uuid::new_v4();
        let new_tenant = TenantModel {
            id: tenant_id,
            name: tenant.name,
            description: tenant.description,
            meta: tenant.meta.map(|m| to_value(&m).unwrap()),
            status: Some(
                tenant
                    .status
                    .into_iter()
                    .map(|s| to_value(&s).unwrap())
                    .collect(),
            ),
            created: Local::now().naive_utc(),
            updated: None,
        };

        let created = conn
            .transaction(|conn| {
                // Inserir o tenant
                let tenant_record = diesel::insert_into(tenant_model::table)
                    .values(&new_tenant)
                    .get_result::<TenantModel>(conn)?;

                // Criar as relações owner_on_tenant para todos os owners
                let owner_records: Vec<OwnerOnTenantModel> = match tenant.owners
                {
                    Children::Records(owners) => owners
                        .iter()
                        .map(|owner| OwnerOnTenantModel {
                            id: Uuid::new_v4(),
                            tenant_id: tenant_record.id.clone(),
                            owner_id: owner.id.clone(),
                            guest_by: guest_by.clone(),
                            created: Local::now().naive_utc(),
                            updated: None,
                        })
                        .collect(),
                    Children::Ids(ids) => ids
                        .iter()
                        .map(|id| OwnerOnTenantModel {
                            id: Uuid::new_v4(),
                            tenant_id: tenant_record.id.clone(),
                            owner_id: id.clone(),
                            guest_by: guest_by.clone(),
                            created: Local::now().naive_utc(),
                            updated: None,
                        })
                        .collect(),
                };

                diesel::insert_into(owner_on_tenant_model::table)
                    .values(&owner_records)
                    .execute(conn)?;

                Ok::<TenantModel, diesel::result::Error>(tenant_record)
            })
            .map_err(|e| {
                creation_err(format!("Failed to create tenant: {}", e))
            })?;

        Ok(CreateResponseKind::Created(Tenant {
            id: Some(created.id),
            name: created.name,
            description: created.description,
            meta: created.meta.map(|m| serde_json::from_value(m).unwrap()),
            status: created
                .status
                .unwrap_or_default()
                .into_iter()
                .map(|s| serde_json::from_value(s).unwrap())
                .collect(),
            created: created.created.and_local_timezone(Local).unwrap(),
            updated: created
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
            owners: Children::Records(vec![]),
            manager: None,
            tags: None,
        }))
    }

    #[tracing::instrument(name = "register_tenant_meta", skip_all)]
    async fn register_tenant_meta(
        &self,
        owners_ids: Vec<Uuid>,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Verify if the tenant exists and if the user has permission
        let tenant = tenant_model::table
            .inner_join(owner_on_tenant_model::table)
            .filter(tenant_model::id.eq(tenant_id))
            .filter(
                owner_on_tenant_model::owner_id.eq_any(
                    owners_ids
                        .iter()
                        .map(|id| id.clone())
                        .collect::<Vec<Uuid>>(),
                ),
            )
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check tenant: {}", e))
            })?;

        let tenant = match tenant {
            Some(t) => t,
            None => {
                return Ok(CreateResponseKind::NotCreated(
                    HashMap::new(),
                    "Tenant not found or user not authorized".to_string(),
                ))
            }
        };

        // Update the tenant meta
        let mut meta_map: std::collections::HashMap<String, String> = tenant
            .meta
            .map(|m| serde_json::from_value(m).unwrap())
            .unwrap_or_default();

        meta_map.insert(format!("{key}", key = key), value.clone());

        diesel::update(tenant_model::table.find(tenant_id))
            .set(tenant_model::meta.eq(to_value(&meta_map).unwrap()))
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to update tenant meta: {}", e))
            })?;

        Ok(CreateResponseKind::Created(meta_map))
    }
}
