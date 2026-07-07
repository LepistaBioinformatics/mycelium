use super::map_tenant_model_to_dto;
use crate::{
    config::SqliteDbPoolProvider,
    models::{
        owner_on_tenant::OwnerOnTenant as OwnerOnTenantModel,
        tenant::Tenant as TenantModel,
    },
    schema::{owner_on_tenant, tenant},
    types::{json_array_to_text, naive_timestamp_to_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantRegistration,
};
use mycelium_base::{
    dtos::Children,
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantRegistration)]
pub struct TenantRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TenantRegistration for TenantRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_tenant", skip_all)]
    async fn create(
        &self,
        tenant_dto: Tenant,
        guest_by: String,
    ) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if a tenant with the same name already exists
        let existing = tenant::table
            .filter(tenant::name.eq(&tenant_dto.name))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing tenant: {}", e))
            })?;

        if let Some(record) = existing {
            return Ok(CreateResponseKind::NotCreated(
                map_tenant_model_to_dto(record),
                "Tenant already exists".to_string(),
            ));
        }

        // Create the new tenant. Each status is individually serialized (one
        // JSON value per entry) so it round-trips correctly through
        // `decode_status` / every fetching and updating read site.
        let status_json = tenant_dto
            .status
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|s| serde_json::to_value(&s).unwrap())
            .collect::<Vec<_>>();

        let new_tenant = TenantModel {
            id: uuid_to_text(&Uuid::new_v4()),
            name: tenant_dto.name,
            description: tenant_dto.description,
            meta: tenant_dto.meta.map(|m| serde_json::to_string(&m).unwrap()),
            status: Some(json_array_to_text(&status_json).map_err(|e| {
                creation_err(format!("Failed to serialize status: {e}"))
            })?),
            created: naive_timestamp_to_text(&Local::now().naive_utc()),
            updated: None,
            encrypted_dek: None,
            kek_version: 1,
        };

        let created: TenantModel = conn
            .transaction(|conn| {
                // Insert the tenant
                let tenant_record = diesel::insert_into(tenant::table)
                    .values(&new_tenant)
                    .returning(TenantModel::as_returning())
                    .get_result::<TenantModel>(conn)?;

                // Create the owner_on_tenant relations for every owner
                let owner_records: Vec<OwnerOnTenantModel> =
                    match tenant_dto.owners {
                        Children::Records(owners) => owners
                            .iter()
                            .map(|owner| OwnerOnTenantModel {
                                id: uuid_to_text(&Uuid::new_v4()),
                                tenant_id: tenant_record.id.clone(),
                                owner_id: uuid_to_text(&owner.id),
                                guest_by: guest_by.clone(),
                                created: naive_timestamp_to_text(
                                    &Local::now().naive_utc(),
                                ),
                                updated: None,
                            })
                            .collect(),
                        Children::Ids(ids) => ids
                            .iter()
                            .map(|id| OwnerOnTenantModel {
                                id: uuid_to_text(&Uuid::new_v4()),
                                tenant_id: tenant_record.id.clone(),
                                owner_id: uuid_to_text(id),
                                guest_by: guest_by.clone(),
                                created: naive_timestamp_to_text(
                                    &Local::now().naive_utc(),
                                ),
                                updated: None,
                            })
                            .collect(),
                    };

                diesel::insert_into(owner_on_tenant::table)
                    .values(&owner_records)
                    .execute(conn)?;

                Ok::<TenantModel, diesel::result::Error>(tenant_record)
            })
            .map_err(|e| {
                creation_err(format!("Failed to create tenant: {}", e))
            })?;

        Ok(CreateResponseKind::Created(map_tenant_model_to_dto(
            created,
        )))
    }

    #[tracing::instrument(name = "register_tenant_meta", skip_all)]
    async fn register_tenant_meta(
        &self,
        owners_ids: Vec<Uuid>,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);
        let owner_ids_text: Vec<String> =
            owners_ids.iter().map(uuid_to_text).collect();

        // Verify if the tenant exists and if the user has permission
        let found = tenant::table
            .inner_join(owner_on_tenant::table)
            .filter(tenant::id.eq(&tenant_id_text))
            .filter(owner_on_tenant::owner_id.eq_any(owner_ids_text))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check tenant: {}", e))
            })?;

        let Some(found) = found else {
            return Ok(CreateResponseKind::NotCreated(
                HashMap::new(),
                "Tenant not found or user not authorized".to_string(),
            ));
        };

        let mut meta_map: HashMap<String, String> = found
            .meta
            .map(|m| serde_json::from_str(&m).unwrap())
            .unwrap_or_default();

        meta_map.insert(format!("{key}", key = key), value.clone());

        let meta_text = serde_json::to_string(&meta_map)
            .expect("meta map is always serializable");

        diesel::update(tenant::table.find(&tenant_id_text))
            .set(tenant::meta.eq(meta_text))
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to update tenant meta: {}", e))
            })?;

        Ok(CreateResponseKind::Created(meta_map))
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::{
            tenant::{
                TenantDeletionSqlDbRepository, TenantFetchingSqlDbRepository,
                TenantUpdatingSqlDbRepository,
            },
            tenant_tag::TenantTagRegistrationSqlDbRepository,
        },
        schema::user,
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::{profile::Owner, tenant::TenantStatus},
        entities::{
            TenantDeletion, TenantFetching, TenantTagRegistration,
            TenantUpdating,
        },
    };
    use mycelium_base::entities::{
        DeletionResponseKind, FetchResponseKind, UpdatingResponseKind,
    };

    #[tokio::test]
    async fn tenant_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let owner_id = Uuid::new_v4();

        // `owner_on_tenant.owner_id` has a FK to `user(id)`; seed a minimal
        // user row directly since the user repositories are not implemented
        // yet (SM-T9).
        {
            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(user::table)
                .values((
                    user::id.eq(uuid_to_text(&owner_id)),
                    user::username.eq("owner"),
                    user::email.eq("owner@acme.test"),
                    user::first_name.eq("Own"),
                    user::last_name.eq("Er"),
                    user::is_active.eq(true),
                    user::created
                        .eq(naive_timestamp_to_text(&Local::now().naive_utc())),
                    user::is_principal.eq(true),
                ))
                .execute(conn)
                .unwrap();
        }

        let registration = TenantRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = TenantFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = TenantUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = TenantDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let tag_registration = TenantTagRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create
        let tenant = Tenant::new_with_owners(
            "Acme Tenant".into(),
            Some("A test tenant".into()),
            Children::Records(vec![Owner {
                id: owner_id,
                email: "owner@acme.test".into(),
                first_name: Some("Own".into()),
                last_name: Some("Er".into()),
                username: Some("owner".into()),
                is_principal: true,
            }]),
        );

        let created = match registration.create(tenant, "self".into()).await? {
            CreateResponseKind::Created(tenant) => tenant,
            CreateResponseKind::NotCreated(..) => {
                panic!("expected the tenant to be created")
            }
        };
        let tenant_id = created.id.expect("created tenant must have an id");
        assert_eq!(created.name, "Acme Tenant");

        // Fetch (owned by me)
        let found = match fetching
            .get_tenant_owned_by_me(tenant_id, vec![owner_id])
            .await?
        {
            FetchResponseKind::Found(tenant) => tenant,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the tenant to be found")
            }
        };
        assert_eq!(found.name, "Acme Tenant");
        assert!(
            matches!(found.owners, Children::Records(ref o) if o.len() == 1)
        );

        // Update name/description
        let mut update_payload = found.clone();
        update_payload.name = "Acme Tenant Updated".into();
        let updated = match updating
            .update_name_and_description(tenant_id, update_payload)
            .await?
        {
            UpdatingResponseKind::Updated(tenant) => tenant,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the tenant to be updated")
            }
        };
        assert_eq!(updated.name, "Acme Tenant Updated");

        // Update status (append)
        let updated = match updating
            .update_tenant_status(
                tenant_id,
                TenantStatus::Archived {
                    at: Local::now(),
                    by: "owner@acme.test".into(),
                },
            )
            .await?
        {
            UpdatingResponseKind::Updated(tenant) => tenant,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the tenant to be updated")
            }
        };
        assert_eq!(updated.status.map(|s| s.len()).unwrap_or(0), 1);

        // Tag
        let tag_outcome = tag_registration
            .get_or_create(tenant_id, "vip".into(), HashMap::new())
            .await?;
        assert!(matches!(
            tag_outcome,
            mycelium_base::entities::GetOrCreateResponseKind::Created(
                ref tag
            ) if tag.value == "vip"
        ));

        // Delete owner
        let owner_deleted = deletion
            .delete_owner(tenant_id, Some(owner_id), None)
            .await?;
        assert!(matches!(owner_deleted, DeletionResponseKind::Deleted));

        // Delete tenant
        let tenant_deleted = deletion.delete(tenant_id).await?;
        assert!(matches!(tenant_deleted, DeletionResponseKind::Deleted));

        Ok(())
    }
}
