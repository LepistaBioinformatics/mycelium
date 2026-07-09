use crate::{
    config::SqliteDbPoolProvider,
    models::tenant::Tenant as TenantModel,
    schema::{owner_on_tenant, tenant, user as users_model},
    types::uuid_to_text,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        email::Email, native_error_codes::NativeErrorCodes,
        tenant::TenantMetaKey,
    },
    entities::TenantDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantDeletion)]
pub struct TenantDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TenantDeletion for TenantDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_tenant", skip_all)]
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let id_text = uuid_to_text(&id);

        // Check if tenant exists
        let exists = tenant::table
            .find(&id_text)
            .select(tenant::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check tenant: {}", e))
            })?;

        match exists {
            Some(_) => {
                // Delete tenant
                diesel::delete(tenant::table.find(&id_text))
                    .execute(conn)
                    .map_err(|e| {
                        deletion_err(format!("Failed to delete tenant: {}", e))
                    })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                id,
                "Tenant not found".to_string(),
            )),
        }
    }

    #[tracing::instrument(name = "delete_tenant_owner", skip_all)]
    async fn delete_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Option<Uuid>,
        owner_email: Option<Email>,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);

        let owner_id_filter = match (owner_id, owner_email) {
            //
            // Delete by owner id
            //
            (Some(id), None) => uuid_to_text(&id),
            //
            // Delete by owner email
            //
            (None, Some(email)) => {
                let user_id = users_model::table
                    .filter(users_model::email.eq(email.email()))
                    .select(users_model::id)
                    .first::<String>(conn)
                    .optional()
                    .map_err(|e| {
                        deletion_err(format!("Failed to fetch user: {}", e))
                    })?;

                match user_id {
                    Some(id) => id,
                    //
                    // Never will be matched with nil uuid
                    //
                    None => uuid_to_text(&Uuid::nil()),
                }
            }
            //
            // Any other case will generate an error
            //
            _ => {
                return deletion_err("Owner ID or email is required")
                    .with_exp_true()
                    .as_error()
            }
        };

        let deleted = diesel::delete(owner_on_tenant::table)
            .filter(owner_on_tenant::tenant_id.eq(&tenant_id_text))
            .filter(owner_on_tenant::owner_id.eq(owner_id_filter))
            .execute(conn)
            .map_err(|e| {
                deletion_err(format!("Failed to delete owner: {}", e))
            })?;

        if deleted > 0 {
            Ok(DeletionResponseKind::Deleted)
        } else {
            Ok(DeletionResponseKind::NotDeleted(
                tenant_id,
                "Owner not found".to_string(),
            ))
        }
    }

    #[tracing::instrument(name = "delete_tenant_meta", skip_all)]
    async fn delete_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);

        let tenant_data = tenant::table
            .find(&tenant_id_text)
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to fetch tenant: {}", e))
            })?;

        let Some(tenant_data) = tenant_data else {
            return Ok(DeletionResponseKind::NotDeleted(
                tenant_id,
                "Tenant not found".to_string(),
            ));
        };

        let Some(meta) = tenant_data.meta else {
            return Ok(DeletionResponseKind::NotDeleted(
                tenant_id,
                "No meta data found".to_string(),
            ));
        };

        let mut meta_map: HashMap<String, String> =
            serde_json::from_str(&meta).unwrap_or_default();

        let Some(_) = meta_map.remove(&format!("{key}", key = key)) else {
            return Ok(DeletionResponseKind::NotDeleted(
                tenant_id,
                "Meta key not found".to_string(),
            ));
        };

        diesel::update(tenant::table.find(&tenant_id_text))
            .set(tenant::meta.eq(serde_json::to_string(&meta_map).unwrap()))
            .execute(conn)
            .map_err(|e| {
                deletion_err(format!("Failed to update tenant meta: {}", e))
            })?;

        Ok(DeletionResponseKind::Deleted)
    }
}
