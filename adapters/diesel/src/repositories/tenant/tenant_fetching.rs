use crate::{
    models::{config::DbConfig, tenant::Tenant as TenantModel},
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
use std::{fmt::Display, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantFetching)]
pub struct TenantFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl TenantFetching for TenantFetchingSqlDbRepository {
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
            .filter(tenant_model::id.eq(id))
            .filter(owner_on_tenant_model::owner_id.eq_any(owners_ids))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        match tenant {
            Some(record) => Ok(FetchResponseKind::Found(Tenant {
                id: Some(record.id),
                name: record.name,
                description: record.description,
                meta: record.meta.map(|m| serde_json::from_value(m).unwrap()),
                status: record
                    .status
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
            .filter(tenant_model::id.eq(id))
            .filter(
                manager_account_on_tenant_model::account_id.eq_any(manager_ids),
            )
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        match tenant {
            Some(record) => Ok(FetchResponseKind::Found(Tenant {
                id: Some(record.id),
                name: record.name,
                description: record.description,
                meta: record.meta.map(|m| serde_json::from_value(m).unwrap()),
                status: record
                    .status
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
                select p.id, p.name, p.is_active, p.is_archived, p.is_trashed, p.meta
                FROM tenant p
                LEFT JOIN owner_on_tenant ot ON p.id = ot.tenant_id
                WHERE (
                    {0} IS NULL OR p.name ILIKE {0}
                    AND {1} IS NULL OR ot.owner_id = {1}
                    AND {2} IS NULL OR p.meta->>{2} = {2}
                    AND {3} IS NULL OR p.status->>'verified' = {3}
                    AND {4} IS NULL OR p.status->>'archived' = {4}
                    AND {5} IS NULL OR p.status->>'trashed' = {5}
                    AND {6} IS NULL OR p.tags->>'{6}' = {7}
                )
                ORDER BY p.created DESC
                LIMIT {8}
                OFFSET {9}
            )
            SELECT
                (SELECT COUNT(*) FROM paginaged) as total,
                ARRAY(SELECT * FROM paginaged) as records
            "#,
            option_to_null(name),
            option_to_null(owner),
            option_to_null(metadata_key),
            option_to_null(status_verified),
            option_to_null(status_archived),
            option_to_null(status_trashed),
            option_to_null(tag_value),
            option_to_null(tag_meta),
            page_size.unwrap_or(10),
            offset,
        );

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
