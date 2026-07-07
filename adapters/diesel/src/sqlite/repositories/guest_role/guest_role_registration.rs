use super::map_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::guest_role::GuestRole as GuestRoleModel,
    schema::guest_role,
    types::{naive_timestamp_to_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{guest_role::GuestRole, native_error_codes::NativeErrorCodes},
    entities::GuestRoleRegistration,
};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleRegistration)]
pub struct GuestRoleRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl GuestRoleRegistration for GuestRoleRegistrationSqlDbRepository {
    #[tracing::instrument(name = "get_or_create_guest_role", skip_all)]
    async fn get_or_create(
        &self,
        guest_role_dto: GuestRole,
    ) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if role already exists
        let existing = guest_role::table
            .filter(guest_role::slug.eq(&guest_role_dto.slug).and(
                guest_role::permission.eq(&guest_role_dto.permission.to_i32()),
            ))
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing role: {}", e))
            })?;

        if let Some(record) = existing {
            return Ok(GetOrCreateResponseKind::NotCreated(
                map_model_to_dto(record),
                "Role already exists".to_string(),
            ));
        }

        // Create new role
        let new_role = GuestRoleModel {
            id: uuid_to_text(&Uuid::new_v4()),
            name: guest_role_dto.name,
            slug: guest_role_dto.slug,
            description: guest_role_dto.description,
            permission: guest_role_dto.permission.to_i32(),
            system: guest_role_dto.system,
            created: naive_timestamp_to_text(&Utc::now().naive_utc()),
            updated: None,
        };

        let created = diesel::insert_into(guest_role::table)
            .values(&new_role)
            .returning(GuestRoleModel::as_returning())
            .get_result(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create role: {}", e))
            })?;

        Ok(GetOrCreateResponseKind::Created(map_model_to_dto(created)))
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::{
        repositories::guest_role::{
            GuestRoleDeletionSqlDbRepository, GuestRoleFetchingSqlDbRepository,
            GuestRoleUpdatingSqlDbRepository,
        },
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::guest_role::Permission,
        entities::{GuestRoleDeletion, GuestRoleFetching, GuestRoleUpdating},
    };
    use mycelium_base::{
        dtos::Children,
        entities::{
            DeletionResponseKind, FetchResponseKind, UpdatingResponseKind,
        },
    };

    #[tokio::test]
    async fn guest_role_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();

        let registration = GuestRoleRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = GuestRoleFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = GuestRoleUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = GuestRoleDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create parent role
        let parent_dto = GuestRole::new(
            None,
            "Viewer".into(),
            Some("Read-only role".into()),
            Permission::Read,
            None,
            false,
        );
        let parent = match registration.get_or_create(parent_dto).await? {
            GetOrCreateResponseKind::Created(role) => role,
            GetOrCreateResponseKind::NotCreated(..) => {
                panic!("expected the role to be created")
            }
        };
        let parent_id = parent.id.expect("created role must have an id");

        // Create child role
        let child_dto = GuestRole::new(
            None,
            "Editor".into(),
            None,
            Permission::Write,
            None,
            false,
        );
        let child = match registration.get_or_create(child_dto).await? {
            GetOrCreateResponseKind::Created(role) => role,
            GetOrCreateResponseKind::NotCreated(..) => {
                panic!("expected the role to be created")
            }
        };
        let child_id = child.id.expect("created role must have an id");

        // Fetch
        let found = match fetching.get(parent_id).await? {
            FetchResponseKind::Found(role) => role,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the role to be found")
            }
        };
        assert_eq!(found.name, "Viewer");

        // Update
        let mut update_payload = found.clone();
        update_payload.name = "Viewer Updated".into();
        let updated = match updating.update(update_payload).await? {
            UpdatingResponseKind::Updated(role) => role,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the role to be updated")
            }
        };
        assert_eq!(updated.name, "Viewer Updated");

        // Insert role child
        let creator_id = Uuid::new_v4();
        let with_child = updating
            .insert_role_child(parent_id, child_id, creator_id)
            .await?;
        assert!(matches!(with_child, UpdatingResponseKind::Updated(_)));

        let found_with_child = match fetching.get(parent_id).await? {
            FetchResponseKind::Found(role) => role,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the role to be found")
            }
        };
        assert!(matches!(
            found_with_child.children,
            Some(Children::Ids(ref ids)) if ids == &vec![child_id]
        ));

        // get_parent_by_child_id
        let parent_of_child =
            match fetching.get_parent_by_child_id(child_id).await? {
                FetchResponseKind::Found(role) => role,
                FetchResponseKind::NotFound(_) => {
                    panic!("expected the parent role to be found")
                }
            };
        assert_eq!(parent_of_child.id, Some(parent_id));

        // Remove role child
        let removed = updating
            .remove_role_child(parent_id, child_id, creator_id)
            .await?;
        assert!(matches!(removed, UpdatingResponseKind::Updated(_)));

        // Delete
        let deleted = deletion.delete(child_id).await?;
        assert!(matches!(deleted, DeletionResponseKind::Deleted));

        Ok(())
    }
}
