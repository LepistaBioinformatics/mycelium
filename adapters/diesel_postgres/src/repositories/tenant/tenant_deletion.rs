use crate::{
    models::{config::DbPoolProvider, tenant::Tenant as TenantModel},
    schema::{
        owner_on_tenant as owner_on_tenant_model, tenant as tenant_model,
        user as users_model,
    },
};

use async_trait::async_trait;
use diesel::{prelude::*, QueryDsl};
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
    pub db_config: Arc<dyn DbPoolProvider>,
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

        // Check if tenant exists
        let tenant = tenant_model::table
            .find(id)
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check tenant: {}", e))
            })?;

        match tenant {
            Some(_) => {
                // Delete tenant
                diesel::delete(tenant_model::table.find(id))
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

        let delete_statement = diesel::delete(owner_on_tenant_model::table)
            .filter(owner_on_tenant_model::tenant_id.eq(tenant_id))
            .filter(match (owner_id, owner_email) {
                //
                // Delete by owner id
                //
                (Some(id), None) => owner_on_tenant_model::owner_id.eq(id),
                //
                // Delete by owner email
                //
                (None, Some(email)) => {
                    let user_id = users_model::table
                        .filter(users_model::email.eq(email.email()))
                        .select(users_model::id)
                        .first::<Uuid>(conn)
                        .optional()
                        .map_err(|e| {
                            deletion_err(format!("Failed to fetch user: {}", e))
                        })?;

                    match user_id {
                        Some(id) => owner_on_tenant_model::owner_id.eq(id),
                        //
                        // Never will be matched with nill uuid
                        //
                        None => owner_on_tenant_model::owner_id.eq(Uuid::nil()),
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
            });

        let deleted = delete_statement.execute(conn).map_err(|e| {
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

        let tenant = tenant_model::table
            .find(tenant_id)
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to fetch tenant: {}", e))
            })?;

        match tenant {
            Some(mut tenant_data) => {
                if let Some(meta) = tenant_data.meta.as_mut() {
                    let mut meta_map: HashMap<String, String> =
                        serde_json::from_value(meta.clone())
                            .unwrap_or_default();

                    if meta_map.remove(&format!("{key}", key = key)).is_some() {
                        diesel::update(tenant_model::table.find(tenant_id))
                            .set(
                                tenant_model::meta
                                    .eq(serde_json::to_value(&meta_map)
                                        .unwrap()),
                            )
                            .execute(conn)
                            .map_err(|e| {
                                deletion_err(format!(
                                    "Failed to update tenant meta: {}",
                                    e
                                ))
                            })?;

                        Ok(DeletionResponseKind::Deleted)
                    } else {
                        Ok(DeletionResponseKind::NotDeleted(
                            tenant_id,
                            "Meta key not found".to_string(),
                        ))
                    }
                } else {
                    Ok(DeletionResponseKind::NotDeleted(
                        tenant_id,
                        "No meta data found".to_string(),
                    ))
                }
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                tenant_id,
                "Tenant not found".to_string(),
            )),
        }
    }
}
