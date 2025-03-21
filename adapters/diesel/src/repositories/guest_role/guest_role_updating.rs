use crate::{
    models::{config::DbPoolProvider, guest_role::GuestRole as GuestRoleModel},
    schema::{guest_role as guest_role_model, guest_role_children},
};

use super::shared::map_model_to_dto;
use async_trait::async_trait;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error},
};
use myc_core::domain::{
    dtos::{guest_role::GuestRole, native_error_codes::NativeErrorCodes},
    entities::GuestRoleUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleUpdating)]
pub struct GuestRoleUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl GuestRoleUpdating for GuestRoleUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_guest_role", skip_all)]
    async fn update(
        &self,
        user_role: GuestRole,
    ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let role_id = user_role.id.ok_or_else(|| {
            updating_err("Role ID is required for update".to_string())
        })?;

        let updated = diesel::update(guest_role_model::table.find(role_id))
            .set((
                guest_role_model::name.eq(&user_role.name),
                guest_role_model::slug.eq(&user_role.slug),
                guest_role_model::description.eq(user_role.description.clone()),
                guest_role_model::permission.eq(user_role.permission.to_i32()),
                guest_role_model::updated.eq(chrono::Utc::now()),
            ))
            .get_result::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to update role: {}", e))
            })?;

        match updated {
            Some(record) => {
                Ok(UpdatingResponseKind::Updated(map_model_to_dto(record)))
            }
            None => Ok(UpdatingResponseKind::NotUpdated(
                user_role,
                "Role not found".to_string(),
            )),
        }
    }

    #[tracing::instrument(name = "insert_role_child", skip_all)]
    async fn insert_role_child(
        &self,
        role_id: Uuid,
        child_id: Uuid,
    ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if both roles exist
        let parent_role = guest_role_model::table
            .find(role_id)
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch parent role: {}", e))
            })?;

        let child_role = guest_role_model::table
            .find(child_id)
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch child role: {}", e))
            })?;

        match (parent_role, child_role) {
            (Some(parent), Some(_)) => {
                // Insert into guest_role_children table
                diesel::insert_into(guest_role_children::table)
                    .values((
                        guest_role_children::parent_id.eq(role_id),
                        guest_role_children::child_role_id.eq(child_id),
                    ))
                    .execute(conn)
                    .map_err(|e| match e {
                        Error::DatabaseError(
                            DatabaseErrorKind::UniqueViolation,
                            _,
                        ) => updating_err("Role hierarchy already exists")
                            .with_code(NativeErrorCodes::MYC00018)
                            .with_exp_true(),
                        _ => {
                            error!("Failed to insert role hierarchy: {}", e);

                            updating_err("Failed to insert role child")
                        }
                    })?;

                Ok(UpdatingResponseKind::Updated(Some(map_model_to_dto(
                    parent,
                ))))
            }
            (None, _) => Ok(UpdatingResponseKind::NotUpdated(
                None,
                "Parent role not found".to_string(),
            )),
            (_, None) => Ok(UpdatingResponseKind::NotUpdated(
                None,
                "Child role not found".to_string(),
            )),
        }
    }

    #[tracing::instrument(name = "remove_role_child", skip_all)]
    async fn remove_role_child(
        &self,
        role_id: Uuid,
        child_id: Uuid,
    ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if parent role exists
        let parent_role = guest_role_model::table
            .find(role_id)
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                updating_err(format!("Failed to fetch parent role: {}", e))
            })?;

        match parent_role {
            Some(parent) => {
                // Remove from guest_role_children table
                let deleted = diesel::delete(
                    guest_role_children::table
                        .filter(guest_role_children::parent_id.eq(role_id))
                        .filter(
                            guest_role_children::child_role_id.eq(child_id),
                        ),
                )
                .execute(conn)
                .map_err(|e| {
                    error!("Failed to remove role child: {}", e);

                    updating_err("Failed to remove role child")
                })?;

                if deleted > 0 {
                    Ok(UpdatingResponseKind::Updated(Some(map_model_to_dto(
                        parent,
                    ))))
                } else {
                    Ok(UpdatingResponseKind::NotUpdated(
                        None,
                        "Child role relationship not found".to_string(),
                    ))
                }
            }
            None => Ok(UpdatingResponseKind::NotUpdated(
                None,
                "Parent role not found".to_string(),
            )),
        }
    }
}
