use crate::{
    models::{
        account::Account as AccountModel, config::DbPoolProvider,
        owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
        tenant::Tenant as TenantModel, tenant_tag::TenantTag as TenantTagModel,
        user::User as UserModel,
    },
    repositories::account::map_account_model_to_dto,
    schema::{
        account::{self as account_model},
        manager_account_on_tenant::{self as manager_account_on_tenant_model},
        owner_on_tenant::{
            self as owner_on_tenant_model, dsl as owner_on_tenant_dsl,
        },
        tenant::{self as tenant_model, dsl as tenant_dsl},
        tenant_tag::dsl as tenant_tag_dsl,
        user::{self as user_model},
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{
    dsl::{sql, AsExprOf},
    expression::{SqlLiteral, UncheckedBind},
    prelude::*,
    sql_types::{Bool, Text},
    BelongingToDsl, QueryDsl,
};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Owner,
        tag::Tag,
        tenant::{Tenant, TenantMetaKey, TenantTagFilter},
    },
    entities::TenantFetching,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::json;
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
            Some(record) => {
                let owners = OwnerOnTenantModel::belonging_to(&record)
                    .inner_join(user_model::table)
                    .select(UserModel::as_select())
                    .load::<UserModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch owners: {}", e))
                    })?
                    .into_iter()
                    .map(|u| Owner {
                        id: u.id,
                        email: u.email,
                        first_name: Some(u.first_name),
                        last_name: Some(u.last_name),
                        username: Some(u.username),
                        is_principal: u.is_principal,
                    })
                    .collect::<Vec<Owner>>();

                let manager = manager_account_on_tenant_model::table
                    .inner_join(account_model::table)
                    .filter(manager_account_on_tenant_model::tenant_id.eq(id))
                    .select(AccountModel::as_select())
                    .first::<AccountModel>(conn)
                    .optional()
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch manager: {e}"))
                    })?;

                let tags = TenantTagModel::belonging_to(&record)
                    .select(TenantTagModel::as_select())
                    .load::<TenantTagModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch tags: {}", e))
                    })?
                    .into_iter()
                    .map(|t| Tag {
                        id: t.id,
                        value: t.value,
                        meta: t
                            .meta
                            .map(|m| serde_json::from_value(m).unwrap()),
                    })
                    .collect::<Vec<Tag>>();

                Ok(FetchResponseKind::Found(Tenant {
                    id: Some(record.id),
                    name: record.name,
                    description: record.description,
                    meta: record.meta.map(|m| {
                        serde_json::from_value::<HashMap<String, String>>(m)
                            .unwrap()
                            .iter()
                            .map(|(k, v)| {
                                (
                                    TenantMetaKey::from_str(k).unwrap(),
                                    v.to_string(),
                                )
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
                    owners: match owners.len() {
                        0 => Children::Records(vec![]),
                        _ => Children::Records(owners),
                    },
                    manager: match manager {
                        None => None,
                        Some(account) => Some(Parent::Record(
                            map_account_model_to_dto(account),
                        )),
                    },
                    tags: match tags.len() {
                        0 => None,
                        _ => Some(tags),
                    },
                }))
            }
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

        let record =
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

        match record {
            Some(record) => {
                let owners = OwnerOnTenantModel::belonging_to(&record)
                    .inner_join(user_model::table)
                    .select(UserModel::as_select())
                    .load::<UserModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch owners: {}", e))
                    })?
                    .into_iter()
                    .map(|u| Owner {
                        id: u.id,
                        email: u.email,
                        first_name: Some(u.first_name),
                        last_name: Some(u.last_name),
                        username: Some(u.username),
                        is_principal: u.is_principal,
                    })
                    .collect::<Vec<Owner>>();

                let manager = manager_account_on_tenant_model::table
                    .inner_join(account_model::table)
                    .filter(manager_account_on_tenant_model::tenant_id.eq(id))
                    .select(AccountModel::as_select())
                    .first::<AccountModel>(conn)
                    .optional()
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch manager: {e}"))
                    })?;

                let tags = TenantTagModel::belonging_to(&record)
                    .select(TenantTagModel::as_select())
                    .load::<TenantTagModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!("Failed to fetch tags: {}", e))
                    })?
                    .into_iter()
                    .map(|t| Tag {
                        id: t.id,
                        value: t.value,
                        meta: t
                            .meta
                            .map(|m| serde_json::from_value(m).unwrap()),
                    })
                    .collect::<Vec<Tag>>();

                Ok(FetchResponseKind::Found(Tenant {
                    id: Some(record.id),
                    name: record.name,
                    description: record.description,
                    meta: record.meta.map(|m| {
                        serde_json::from_value::<HashMap<String, String>>(m)
                            .unwrap()
                            .iter()
                            .map(|(k, v)| {
                                (
                                    TenantMetaKey::from_str(k).unwrap(),
                                    v.to_string(),
                                )
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
                    owners: match owners.len() {
                        0 => Children::Records(vec![]),
                        _ => Children::Records(owners),
                    },
                    manager: match manager {
                        None => None,
                        Some(account) => Some(Parent::Record(
                            map_account_model_to_dto(account),
                        )),
                    },
                    tags: match tags.len() {
                        0 => None,
                        _ => Some(tags),
                    },
                }))
            }
            None => Ok(FetchResponseKind::NotFound(Some(id.to_string()))),
        }
    }

    #[tracing::instrument(name = "filter_tenants_as_manager", skip_all)]
    async fn filter_tenants_as_manager(
        &self,
        name: Option<String>,
        owner: Option<Uuid>,
        metadata: Option<(TenantMetaKey, String)>,
        tag: TenantTagFilter,
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

        if let Some(tag_value) = tag.value {
            let dsl = tenant_tag_dsl::value.eq(tag_value);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some((meta_key, meta_value)) = tag.meta {
            let dsl = tenant_tag_dsl::meta.contains(json!({
                meta_key: meta_value
            }));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some((meta_key, value)) = metadata {
            let dsl = tenant_metadata_containment(&meta_key, &value)?;
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        // Get total of records
        let total = count_query
            .select(diesel::dsl::count(tenant_dsl::id).aggregate_distinct())
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

type TenantMetadataContainment =
    SqlLiteral<Bool, UncheckedBind<SqlLiteral<Bool>, AsExprOf<String, Text>>>;

/// Builds the case-insensitive `tenant.meta @> {key: value}` filter.
///
/// The JSON document is sent as a bind parameter: both the key (through
/// `TenantMetaKey::Custom`) and the value are client-controlled, so they must
/// never be interpolated into the SQL text.
fn tenant_metadata_containment(
    meta_key: &TenantMetaKey,
    value: &str,
) -> Result<TenantMetadataContainment, MappedErrors> {
    let document = serde_json::to_string(&json!({
        meta_key.to_string().to_lowercase(): value
    }))
    .map_err(|e| {
        fetching_err(format!("Failed to serialize metadata filter: {e}"))
    })?;

    Ok(sql::<Bool>("LOWER(tenant.meta::text)::jsonb @> LOWER(")
        .bind::<Text, _>(document)
        .sql(")::jsonb"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{debug_query, pg::Pg};

    #[test]
    fn metadata_filter_binds_quotes_instead_of_interpolating_them() {
        let injection = "x') OR (SELECT pg_sleep(5)) IS NULL OR ('";
        let filter = tenant_metadata_containment(
            &TenantMetaKey::Custom(injection.to_string()),
            injection,
        )
        .unwrap();

        let rendered = debug_query::<Pg, _>(&filter).to_string();
        let (sql_text, binds) = rendered.split_once(" -- binds: ").unwrap();

        assert_eq!(
            sql_text,
            "LOWER(tenant.meta::text)::jsonb @> LOWER($1)::jsonb"
        );
        assert!(!sql_text.contains('\''));
        assert!(binds.contains("pg_sleep"));
    }
}
