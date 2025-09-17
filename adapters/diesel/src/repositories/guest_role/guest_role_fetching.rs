use super::shared::map_model_to_dto;
use crate::{
    models::{
        config::DbPoolProvider, guest_role::GuestRole as GuestRoleModel,
        guest_role_children::GuestRoleChildren as GuestRoleChildrenModel,
    },
    schema::guest_role as guest_role_model,
};

use async_trait::async_trait;
use diesel::{prelude::*, BelongingToDsl};
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
    pub db_config: Arc<dyn DbPoolProvider>,
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

        let role = guest_role_model::table
            .find(id)
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch role: {}", e))
            })?;

        if let Some(role) = role {
            let children = GuestRoleChildrenModel::belonging_to(&role)
                .select(GuestRoleChildrenModel::as_select())
                .load::<GuestRoleChildrenModel>(conn)
                .map_err(|e| {
                    fetching_err(format!("Failed to fetch children: {}", e))
                })?;

            let mut role = map_model_to_dto(role);

            role.children = if children.is_empty() {
                None
            } else {
                Some(Children::Ids(
                    children.into_iter().map(|c| c.child_role_id).collect(),
                ))
            };

            return Ok(FetchResponseKind::Found(role));
        }

        Ok(FetchResponseKind::NotFound(Some(id)))
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

        let parent_role = guest_role_model::table
            .inner_join(
                crate::schema::guest_role_children::table
                    .on(crate::schema::guest_role_children::child_role_id
                        .eq(guest_role_model::id)),
            )
            .filter(crate::schema::guest_role_children::child_role_id.eq(id))
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch parent role: {}", e))
            })?;

        if let Some(role) = parent_role {
            let children = GuestRoleChildrenModel::belonging_to(&role)
                .select(GuestRoleChildrenModel::as_select())
                .load::<GuestRoleChildrenModel>(conn)
                .map_err(|e| {
                    fetching_err(format!("Failed to fetch children: {}", e))
                })?;
            let mut role = map_model_to_dto(role);
            role.children = if children.is_empty() {
                None
            } else {
                Some(Children::Ids(
                    children.into_iter().map(|c| c.child_role_id).collect(),
                ))
            };

            return Ok(FetchResponseKind::Found(role));
        }

        Ok(FetchResponseKind::NotFound(Some(id)))
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

        let mut query_records = guest_role_model::table.into_boxed();
        let mut query_count = guest_role_model::table.into_boxed();

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        // Apply name filter if provided
        if let Some(name) = name {
            let stm = guest_role_model::name.ilike(format!("%{}%", name));
            query_records = query_records.filter(stm.to_owned());
            query_count = query_count.filter(stm);
        }

        // Apply slug filter if provided
        if let Some(slug) = slug {
            let stm = guest_role_model::slug.ilike(format!("%{}%", slug));
            query_records = query_records.filter(stm.to_owned());
            query_count = query_count.filter(stm);
        }

        // Apply system filter if provided
        if let Some(system) = system {
            let stm = guest_role_model::system.eq(system);
            query_records = query_records.filter(stm.to_owned());
            query_count = query_count.filter(stm);
        }

        let total = query_count.count().first::<i64>(conn).map_err(|e| {
            fetching_err(format!("Failed to fetch roles: {}", e))
        })?;

        let records = query_records
            .select(GuestRoleModel::as_select())
            .order((
                guest_role_model::updated.desc().nulls_last(),
                guest_role_model::system.desc(),
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

                role.children = if children.is_empty() {
                    None
                } else {
                    Some(Children::Ids(
                        children.into_iter().map(|c| c.child_role_id).collect(),
                    ))
                };

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
