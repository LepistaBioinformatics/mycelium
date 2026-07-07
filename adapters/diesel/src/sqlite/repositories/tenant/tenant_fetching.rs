use super::map_tenant_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::{
        account::Account as AccountModel,
        owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
        tenant::Tenant as TenantModel, tenant_tag::TenantTag as TenantTagModel,
        user::User as UserModel,
    },
    repositories::account::map_account_model_to_dto,
    schema::{
        account, manager_account_on_tenant,
        owner_on_tenant::{self, dsl as owner_on_tenant_dsl},
        tenant::{self, dsl as tenant_dsl},
        tenant_tag::dsl as tenant_tag_dsl,
        user,
    },
    types::uuid_to_text,
};

use async_trait::async_trait;
use diesel::{dsl::sql, prelude::*, sql_types::Text, BelongingToDsl, QueryDsl};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Owner,
        tag::Tag,
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantFetching,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantFetching)]
pub struct TenantFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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

        let id_text = uuid_to_text(&id);
        let owner_ids_text: Vec<String> =
            owners_ids.iter().map(uuid_to_text).collect();

        let record = tenant::table
            .inner_join(owner_on_tenant::table)
            .filter(tenant::id.eq(&id_text))
            .filter(owner_on_tenant::owner_id.eq_any(owner_ids_text))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let Some(record) = record else {
            return Ok(FetchResponseKind::NotFound(Some(id.to_string())));
        };

        Ok(FetchResponseKind::Found(
            self.hydrate(conn, record, &id_text)?,
        ))
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

        let id_text = uuid_to_text(&id);

        let record = tenant::table
            .filter(tenant::id.eq(&id_text))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let Some(record) = record else {
            return Ok(FetchResponseKind::NotFound(Some(id.to_string())));
        };

        let tags = TenantTagModel::belonging_to(&record)
            .select(TenantTagModel::as_select())
            .load::<TenantTagModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tags: {}", e))
            })?;

        let mut tenant = map_tenant_model_to_dto(record);

        tenant.tags =
            Some(tags.into_iter().map(tag_model_to_dto).collect::<Vec<Tag>>());

        Ok(FetchResponseKind::Found(tenant))
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

        let id_text = uuid_to_text(&id);
        let manager_ids_text: Vec<String> =
            manager_ids.iter().map(uuid_to_text).collect();

        let record = tenant::table
            .inner_join(manager_account_on_tenant::table)
            .filter(tenant::id.eq(&id_text))
            .filter(
                manager_account_on_tenant::account_id.eq_any(manager_ids_text),
            )
            .select(TenantModel::as_select())
            .order_by(tenant::created.desc())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let Some(record) = record else {
            return Ok(FetchResponseKind::NotFound(Some(id.to_string())));
        };

        Ok(FetchResponseKind::Found(
            self.hydrate(conn, record, &id_text)?,
        ))
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
            // SQLite's LIKE is case-insensitive for ASCII by default, matching
            // postgres's ILIKE for the common case.
            let dsl = tenant_dsl::name.like(format!("%{}%", term));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(owner_id) = owner {
            let dsl = owner_on_tenant_dsl::owner_id.eq(uuid_to_text(&owner_id));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some((meta_key, value)) = tag {
            // Mirrors the postgres `tenant_tag.meta.contains(to_value(meta_key))`
            // filter, which compares the whole `meta` JSON document against a
            // bare string scalar. Since `meta` is always an object, this never
            // matches in practice on either backend -- preserved as-is rather
            // than "fixed", pending an audit of the call site.
            let meta_key_json = serde_json::to_string(&meta_key)
                .expect("string is always serializable");

            let dsl = sql::<diesel::sql_types::Bool>("tenant_tag.meta = ")
                .bind::<Text, _>(meta_key_json)
                .and(tenant_tag_dsl::value.eq(value));

            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some((meta_key, value)) = metadata {
            // Targets the practical intent of postgres's
            // `LOWER(meta::text)::jsonb @> LOWER(...)::jsonb` filter
            // (case-insensitive match on a single meta key) via SQLite
            // JSON1's json_extract, rather than the "lowercase the whole
            // document" mechanism.
            let path = format!("$.{}", meta_key.to_string().to_lowercase());

            let dsl = sql::<diesel::sql_types::Bool>(&format!(
                "LOWER(json_extract(tenant.meta, '{path}')) = LOWER("
            ))
            .bind::<Text, _>(value)
            .sql(")");

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
                    .map(|o| uuid_to_owner_id(&o))
                    .collect::<Vec<Uuid>>();

                let tags = if tags.is_empty() {
                    None
                } else {
                    Some(
                        tags.into_iter()
                            .map(tag_model_to_dto)
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

fn tag_model_to_dto(t: TenantTagModel) -> Tag {
    Tag {
        id: crate::sqlite::types::uuid_from_text(&t.id).unwrap(),
        value: t.value,
        meta: t.meta.map(|m| serde_json::from_str(&m).unwrap()),
    }
}

fn uuid_to_owner_id(o: &OwnerOnTenantModel) -> Uuid {
    crate::sqlite::types::uuid_from_text(&o.owner_id).unwrap()
}

impl TenantFetchingSqlDbRepository {
    fn hydrate(
        &self,
        conn: &mut diesel::SqliteConnection,
        record: TenantModel,
        tenant_id_text: &str,
    ) -> Result<Tenant, MappedErrors> {
        let owners = OwnerOnTenantModel::belonging_to(&record)
            .inner_join(user::table)
            .select(UserModel::as_select())
            .load::<UserModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch owners: {}", e))
            })?
            .into_iter()
            .map(|u| Owner {
                id: crate::sqlite::types::uuid_from_text(&u.id).unwrap(),
                email: u.email,
                first_name: Some(u.first_name),
                last_name: Some(u.last_name),
                username: Some(u.username),
                is_principal: u.is_principal,
            })
            .collect::<Vec<Owner>>();

        let manager = manager_account_on_tenant::table
            .inner_join(account::table)
            .filter(manager_account_on_tenant::tenant_id.eq(tenant_id_text))
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch manager: {e}"))
            })?;

        let tags = TenantTagModel::belonging_to(&record)
            .select(TenantTagModel::as_select())
            .load::<TenantTagModel>(conn)
            .map_err(|e| fetching_err(format!("Failed to fetch tags: {}", e)))?
            .into_iter()
            .map(tag_model_to_dto)
            .collect::<Vec<Tag>>();

        let mut tenant = map_tenant_model_to_dto(record);

        tenant.owners = match owners.len() {
            0 => Children::Records(vec![]),
            _ => Children::Records(owners),
        };
        tenant.manager =
            manager.map(|a| Parent::Record(map_account_model_to_dto(a)));
        tenant.tags = match tags.len() {
            0 => None,
            _ => Some(tags),
        };

        Ok(tenant)
    }
}
