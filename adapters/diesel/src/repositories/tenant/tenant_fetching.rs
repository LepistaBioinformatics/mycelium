use crate::{
    models::{config::DbPoolProvider, tenant::Tenant as TenantModel},
    schema::{
        manager_account_on_tenant as manager_account_on_tenant_model,
        owner_on_tenant as owner_on_tenant_model, tenant as tenant_model,
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{
    deserialize::QueryableByName,
    prelude::*,
    sql_types::{Array, BigInt, Jsonb},
    QueryDsl,
};
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
use serde_json::Value as JsonValue;
use shaku::Component;
use std::{fmt::Display, str::FromStr, sync::Arc};
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
                    .filter_map(|s| s)
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
                    .filter_map(|s| s)
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
        status_verified: Option<bool>,
        status_archived: Option<bool>,
        status_trashed: Option<bool>,
        tag_value: Option<String>,
        tag_meta: Option<String>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Calculating the offset
        let offset = (skip.unwrap_or(0) * page_size.unwrap_or(10)) as i64;

        // Build the SQL query with WITH
        let sql = format!(
            r#"
            WITH paginaged AS (
                SELECT 
                    p.id, 
                    p.name, 
                    p.tags,
                    p.meta,
                    p.status,
                    p.is_active,
                    p.is_archived,
                    p.is_trashed
                FROM tenant p
                LEFT JOIN owner_on_tenant ot ON p.id = ot.tenant_id
                WHERE (
                    {name} IS NULL OR p.name ILIKE {name}
                    AND {owner} IS NULL OR ot.owner_id = {owner}
                    AND {metadata_key} IS NULL OR p.meta->>{metadata_key} = {metadata_key}
                    AND {status_verified} IS NULL OR p.status->>'verified' = {status_verified}
                    AND {status_archived} IS NULL OR p.status->>'archived' = {status_archived}
                    AND {status_trashed} IS NULL OR p.status->>'trashed' = {status_trashed}
                    AND {tag_meta} IS NULL OR p.tags->>'{tag_meta}' = {tag_value}
                )
                ORDER BY p.created DESC
                LIMIT {page_size}
                OFFSET {offset}
            )
            SELECT
                (SELECT COUNT(*) FROM paginaged) as total,
                ARRAY(SELECT * FROM paginaged) as records
            "#,
            name = option_to_null(name),
            owner = option_to_null(owner),
            metadata_key = option_to_null(metadata_key),
            status_verified = option_to_null(status_verified),
            status_archived = option_to_null(status_archived),
            status_trashed = option_to_null(status_trashed),
            tag_value = option_to_null(tag_value),
            tag_meta = option_to_null(tag_meta),
            page_size = page_size.unwrap_or(10),
            offset = offset,
        );

        println!("{}", sql);

        let results: Vec<TenantWithCount> =
            diesel::sql_query(&sql)
                .bind::<diesel::sql_types::BigInt, _>(
                    page_size.unwrap_or(10) as i64
                )
                .bind::<diesel::sql_types::BigInt, _>(offset)
                .load(conn)
                .map_err(|e| {
                    fetching_err(format!("Failed to execute query: {}", e))
                })?;

        if results.is_empty() {
            return Ok(FetchManyResponseKind::FoundPaginated(
                PaginatedRecord {
                    count: 0,
                    skip: Some(skip.unwrap_or(0) as i64),
                    size: Some(page_size.unwrap_or(10) as i64),
                    records: vec![],
                },
            ));
        }

        let total = if results.is_empty() {
            0
        } else {
            results[0].total
        };

        let records = results[0]
            .records
            .iter()
            .map(|r| serde_json::from_value(r.clone()).unwrap())
            .collect::<Vec<_>>();

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count: total,
            skip: Some(skip.unwrap_or(0) as i64),
            size: Some(page_size.unwrap_or(10) as i64),
            records,
        }))
    }
}

fn option_to_null<T: Display>(value: Option<T>) -> String {
    match value {
        Some(v) => format!("'{}'", v),
        None => "NULL".to_string(),
    }
}

#[derive(QueryableByName)]
struct TenantWithCount {
    #[diesel(sql_type = BigInt)]
    total: i64,
    #[diesel(sql_type = Array<Jsonb>)]
    records: Vec<JsonValue>,
}
