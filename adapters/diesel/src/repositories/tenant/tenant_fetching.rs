use crate::{
    models::{config::DbPoolProvider, tenant::Tenant as TenantModel},
    schema::{
        manager_account_on_tenant::{self as manager_account_on_tenant_model},
        owner_on_tenant::{
            self as owner_on_tenant_model, dsl as owner_on_tenant_dsl,
        },
        tenant::{self as tenant_model, dsl as tenant_dsl},
        tenant_tag::dsl as tenant_tag_dsl,
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{prelude::*, QueryDsl};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantFetching,
};
use mycelium_base::{
    dtos::{Children, PaginatedRecord},
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::to_value;
use shaku::Component;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantFetching)]
pub struct TenantFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TenantFetching for TenantFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_tenant_owned_by_me", skip_all)]
    async fn get_tenant_owned_by_me(
        &self,
        id: Uuid,
        owners_ids: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant = tenant_model::table
            .inner_join(owner_on_tenant_model::table)
            .filter(tenant_model::id.eq(id.to_string()))
            .filter(
                owner_on_tenant_model::owner_id.eq_any(
                    owners_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<String>>(),
                ),
            )
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        match tenant {
            Some(record) => Ok(FetchResponseKind::Found(Tenant {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                meta: record.meta.map(|m| serde_json::from_value(m).unwrap()),
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
            })),
            None => Ok(FetchResponseKind::NotFound(Some(id.to_string()))),
        }
    }

    #[tracing::instrument(name = "get_tenants_by_manager_account", skip_all)]
    async fn get_tenants_by_manager_account(
        &self,
        id: Uuid,
        manager_ids: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant = tenant_model::table
            .inner_join(manager_account_on_tenant_model::table)
            .filter(tenant_model::id.eq(id.to_string()))
            .filter(
                manager_account_on_tenant_model::account_id.eq_any(
                    manager_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<String>>(),
                ),
            )
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        match tenant {
            Some(record) => Ok(FetchResponseKind::Found(Tenant {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                meta: record.meta.map(|m| serde_json::from_value(m).unwrap()),
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
            })),
            None => Ok(FetchResponseKind::NotFound(Some(id.to_string()))),
        }
    }

    #[tracing::instrument(name = "filter_tenants_as_manager", skip_all)]
    async fn filter_tenants_as_manager(
        &self,
        name: Option<String>,
        owner: Option<Uuid>,
        metadata_key: Option<TenantMetaKey>,
        tag: Option<(String, String)>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = tenant_dsl::tenant
            .into_boxed()
            .left_join(tenant_tag_dsl::tenant_tag)
            .left_join(owner_on_tenant_dsl::owner_on_tenant);

        if let Some(term) = name {
            query = query.filter(tenant_dsl::name.ilike(format!("%{}%", term)));
        }

        if let Some(owner_id) = owner {
            query = query
                .filter(owner_on_tenant_dsl::owner_id.eq(owner_id.to_string()));
        }

        if let Some((meta, value)) = tag {
            // Filter by meta
            //
            // Meta is a JSONB column, so we need to filter this field as a string that contains the key
            query = query
                .filter(tenant_tag_dsl::meta.contains(to_value(meta).unwrap()));

            // Filter by value
            query = query.filter(tenant_tag_dsl::value.eq(value));
        }

        if let Some(meta_key) = metadata_key {
            query = query
                .filter(tenant_dsl::meta.contains(to_value(meta_key).unwrap()));
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let offset = skip.unwrap_or(0) as i64;

        let records = query
            .select(TenantModel::as_select())
            .order_by(tenant_dsl::created.desc())
            .limit(page_size)
            .offset(offset)
            .load::<TenantModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenants: {}", e))
            })?;

        let total =
            tenant_dsl::tenant.count().get_result::<i64>(conn).map_err(
                |e| fetching_err(format!("Failed to get total count: {}", e)),
            )?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count: total,
            skip: Some(offset),
            size: Some(page_size),
            records: records.into_iter().map(map_tenant_model_to_dto).collect(),
        }))
    }
}

fn map_tenant_model_to_dto(record: TenantModel) -> Tenant {
    Tenant {
        id: Some(Uuid::from_str(&record.id).unwrap()),
        name: record.name,
        description: record.description,
        meta: record.meta.map(|m| serde_json::from_value(m).unwrap()),
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
    }
}
