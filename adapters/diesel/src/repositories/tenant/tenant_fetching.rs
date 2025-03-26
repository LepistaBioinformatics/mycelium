use crate::{
    models::{
        config::DbPoolProvider,
        owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
        tenant::Tenant as TenantModel, tenant_tag::TenantTag as TenantTagModel,
    },
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
use diesel::{dsl::sql, prelude::*, BelongingToDsl, QueryDsl};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        tag::Tag,
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantFetching,
};
use mycelium_base::{
    dtos::Children,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::{json, to_value};
use shaku::Component;
use std::{collections::HashMap, str::FromStr, sync::Arc};
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
            .filter(tenant_model::id.eq(id))
            .filter(
                owner_on_tenant_model::owner_id
                    .eq_any(owners_ids.iter().map(|id| id).collect::<Vec<_>>()),
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
                meta: record.meta.map(|m| {
                    serde_json::from_value::<HashMap<String, String>>(m)
                        .unwrap()
                        .iter()
                        .map(|(k, v)| {
                            (TenantMetaKey::from_str(k).unwrap(), v.to_string())
                        })
                        .collect()
                }),
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

    #[tracing::instrument(name = "get_tenant_public_by_id", skip_all)]
    async fn get_tenant_public_by_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let record = tenant_model::table
            .filter(tenant_model::id.eq(id))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        match record {
            Some(record) => {
                let tags = TenantTagModel::belonging_to(&record)
                    .select(TenantTagModel::as_select())
                    .load::<TenantTagModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch tags: {}", e))
                    })?;

                let mut tenant = map_tenant_model_to_dto(record);

                let tags = tags
                    .into_iter()
                    .map(|t| Tag {
                        id: t.id,
                        value: t.value,
                        meta: t
                            .meta
                            .map(|m| serde_json::from_value(m).unwrap()),
                    })
                    .collect::<Vec<Tag>>();

                tenant.tags = Some(tags);

                Ok(FetchResponseKind::Found(tenant))
            }
            None => {
                return Ok(FetchResponseKind::NotFound(Some(id.to_string())))
            }
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

        let tenant =
            tenant_model::table
                .inner_join(manager_account_on_tenant_model::table)
                .filter(tenant_model::id.eq(id))
                .filter(manager_account_on_tenant_model::account_id.eq_any(
                    manager_ids.iter().map(|id| id).collect::<Vec<_>>(),
                ))
                .select(TenantModel::as_select())
                .order_by(tenant_model::created.desc())
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
                meta: record.meta.map(|m| {
                    serde_json::from_value::<HashMap<String, String>>(m)
                        .unwrap()
                        .iter()
                        .map(|(k, v)| {
                            (TenantMetaKey::from_str(k).unwrap(), v.to_string())
                        })
                        .collect()
                }),
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
        metadata: Option<(TenantMetaKey, String)>,
        tag: Option<(String, String)>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let base_query = tenant_dsl::tenant
            .inner_join(owner_on_tenant_dsl::owner_on_tenant)
            .left_join(tenant_tag_dsl::tenant_tag);
        let mut count_query = base_query.into_boxed();
        let mut records_query = base_query.into_boxed();

        if let Some(term) = name {
            let dsl = tenant_dsl::name.ilike(format!("%{}%", term));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(owner_id) = owner {
            let dsl = owner_on_tenant_dsl::owner_id.eq(owner_id);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some((meta, value)) = tag {
            let dsl = tenant_tag_dsl::meta
                .contains(to_value(meta).unwrap())
                .and(tenant_tag_dsl::value.eq(value));

            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some((meta_key, value)) = metadata {
            let json_filter = format!(
                "LOWER(tenant.meta::text)::jsonb @> LOWER('{}'::text)::jsonb",
                serde_json::to_string(&json!({
                    meta_key.to_string().to_lowercase(): value.to_owned()
                }))
                .unwrap()
            );

            let dsl = sql::<diesel::sql_types::Bool>(&json_filter);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        // Get total of records
        let total = count_query
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to count tenants: {}", e))
            })?;

        // Get paginated records
        let records = records_query
            .select(TenantModel::as_select())
            .distinct()
            .order_by(tenant_dsl::created.desc())
            .limit(page_size.unwrap_or(10) as i64)
            .offset(skip.unwrap_or(0) as i64)
            .load::<TenantModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenants: {}", e))
            })?;

        let owners = OwnerOnTenantModel::belonging_to(&records)
            .select(OwnerOnTenantModel::as_select())
            .load::<OwnerOnTenantModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch owners: {}", e))
            })?
            .grouped_by(&records);

        let tags = TenantTagModel::belonging_to(&records)
            .select(TenantTagModel::as_select())
            .load::<TenantTagModel>(conn)
            .map_err(|e| fetching_err(format!("Failed to fetch tags: {}", e)))?
            .grouped_by(&records);

        let tenants: Vec<Tenant> = records
            .into_iter()
            .zip(owners)
            .zip(tags)
            .map(|((tenant, owners), tags)| {
                let mut tenant = map_tenant_model_to_dto(tenant);

                let owners = owners
                    .into_iter()
                    .map(|o| o.owner_id)
                    .collect::<Vec<Uuid>>();

                let tags = if tags.is_empty() {
                    None
                } else {
                    Some(
                        tags.into_iter()
                            .map(|t| Tag {
                                id: t.id,
                                value: t.value,
                                meta: t.meta.map(|m| {
                                    serde_json::from_value(m).unwrap()
                                }),
                            })
                            .collect::<Vec<Tag>>(),
                    )
                };

                tenant.owners = Children::Ids(owners);
                tenant.tags = tags;

                tenant
            })
            .collect();

        if tenants.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip.unwrap_or(0) as i64),
            size: Some(page_size.unwrap_or(10) as i64),
            records: tenants,
        })
    }
}

fn map_tenant_model_to_dto(record: TenantModel) -> Tenant {
    Tenant {
        id: Some(record.id),
        name: record.name,
        description: record.description,
        meta: record.meta.map(|m| {
            serde_json::from_value::<HashMap<String, String>>(m)
                .unwrap()
                .iter()
                .map(|(k, v)| {
                    (TenantMetaKey::from_str(k).unwrap(), v.to_string())
                })
                .collect()
        }),
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
