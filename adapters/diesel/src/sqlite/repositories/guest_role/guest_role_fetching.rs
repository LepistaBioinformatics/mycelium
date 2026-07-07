use super::map_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::{
        guest_role::GuestRole as GuestRoleModel,
        guest_role_children::GuestRoleChildren as GuestRoleChildrenModel,
    },
    schema::{guest_role, guest_role_children},
    types::{uuid_from_text, uuid_to_text},
};

use async_trait::async_trait;
use diesel::{dsl::sql, prelude::*, sql_types::Bool, BelongingToDsl};
use myc_core::domain::{
    dtos::{guest_role::GuestRole, native_error_codes::NativeErrorCodes},
    entities::GuestRoleFetching,
};
use mycelium_base::{
    dtos::Children,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleFetching)]
pub struct GuestRoleFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl GuestRoleFetching for GuestRoleFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_guest_role", skip_all)]
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let role = guest_role::table
            .find(uuid_to_text(&id))
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch role: {}", e))
            })?;

        let Some(role) = role else {
            return Ok(FetchResponseKind::NotFound(Some(id)));
        };

        let children = self.load_children(conn, &role)?;
        let mut role = map_model_to_dto(role);
        role.children = children;

        Ok(FetchResponseKind::Found(role))
    }

    #[tracing::instrument(name = "get_parent_by_child_id", skip_all)]
    async fn get_parent_by_child_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let parent_role = guest_role::table
            .inner_join(
                guest_role_children::table
                    .on(guest_role_children::parent_id.eq(guest_role::id)),
            )
            .filter(guest_role_children::child_role_id.eq(uuid_to_text(&id)))
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch parent role: {}", e))
            })?;

        let Some(role) = parent_role else {
            return Ok(FetchResponseKind::NotFound(Some(id)));
        };

        let children = self.load_children(conn, &role)?;
        let mut role = map_model_to_dto(role);
        role.children = children;

        Ok(FetchResponseKind::Found(role))
    }

    #[tracing::instrument(name = "list_guest_roles", skip_all)]
    async fn list(
        &self,
        name: Option<String>,
        slug: Option<String>,
        system: Option<bool>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query_records = guest_role::table.into_boxed();
        let mut query_count = guest_role::table.into_boxed();

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        // SQLite's LIKE is case-insensitive for ASCII by default, matching
        // postgres's ILIKE for the common case.
        if let Some(name) = name {
            let stm = guest_role::name.like(format!("%{}%", name));
            query_records = query_records.filter(stm.to_owned());
            query_count = query_count.filter(stm);
        }

        if let Some(slug) = slug {
            let stm = guest_role::slug.like(format!("%{}%", slug));
            query_records = query_records.filter(stm.to_owned());
            query_count = query_count.filter(stm);
        }

        if let Some(system) = system {
            let stm = guest_role::system.eq(system);
            query_records = query_records.filter(stm.to_owned());
            query_count = query_count.filter(stm);
        }

        let total = query_count.count().first::<i64>(conn).map_err(|e| {
            fetching_err(format!("Failed to fetch roles: {}", e))
        })?;

        // `NULLS LAST` is expressed portably as "is-null ascending" (false=0
        // sorts before true=1), since Diesel's `.nulls_last()` combinator is
        // only implemented for postgres-family backends.
        let records = query_records
            .select(GuestRoleModel::as_select())
            .order((
                sql::<Bool>("updated IS NULL"),
                guest_role::updated.desc(),
                guest_role::system.desc(),
            ))
            .limit(page_size)
            .offset(skip)
            .load::<GuestRoleModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch roles: {}", e))
            })?;

        let children = GuestRoleChildrenModel::belonging_to(&records)
            .select(GuestRoleChildrenModel::as_select())
            .load::<GuestRoleChildrenModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch children: {}", e))
            })?
            .grouped_by(&records);

        let roles = records
            .into_iter()
            .zip(children)
            .map(|(role, children)| {
                let mut role = map_model_to_dto(role);
                role.children = children_to_dto(children);
                role
            })
            .collect();

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip),
            size: Some(page_size),
            records: roles,
        })
    }
}

fn children_to_dto(
    children: Vec<GuestRoleChildrenModel>,
) -> Option<Children<GuestRole, Uuid>> {
    if children.is_empty() {
        return None;
    }

    Some(Children::Ids(
        children
            .into_iter()
            .map(|c| uuid_from_text(&c.child_role_id).unwrap())
            .collect(),
    ))
}

impl GuestRoleFetchingSqlDbRepository {
    fn load_children(
        &self,
        conn: &mut diesel::SqliteConnection,
        role: &GuestRoleModel,
    ) -> Result<Option<Children<GuestRole, Uuid>>, MappedErrors> {
        let children = GuestRoleChildrenModel::belonging_to(role)
            .select(GuestRoleChildrenModel::as_select())
            .load::<GuestRoleChildrenModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch children: {}", e))
            })?;

        Ok(children_to_dto(children))
    }
}
