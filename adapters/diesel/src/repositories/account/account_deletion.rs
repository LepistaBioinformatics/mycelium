use crate::{
    models::{config::DbPoolProvider, internal_error::InternalError},
    schema::{
        account as account_model, account_tag as account_tag_model,
        guest_user_on_account as guest_user_on_account_model,
        manager_account_on_tenant as manager_account_on_tenant_model,
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        account::AccountMetaKey, native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts,
    },
    entities::AccountDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use serde_json::Value as JsonValue;
use shaku::Component;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountDeletion)]
pub struct AccountDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl AccountDeletion for AccountDeletionSqlDbRepository {
    #[tracing::instrument(name = "soft_delete_account", skip_all)]
    async fn soft_delete(
        &self,
        account_id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = account_model::table.into_boxed();

        // Apply related accounts filter if provided
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            let ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
            query = query.filter(
                account_model::id.eq_any(
                    ids.iter()
                        .map(|id| Uuid::from_str(id).unwrap())
                        .collect::<Vec<_>>(),
                ),
            );
        }

        // Check if account exists and is allowed
        let account_exists = query
            .filter(account_model::id.eq(account_id))
            .select(account_model::id)
            .first::<Uuid>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check account: {}", e))
            })?;

        match account_exists {
            Some(_) => {
                let account_id_string = format!("{}-deleted", account_id);

                let transaction_result: Result<(), _> =
                    conn.transaction(|conn| {
                        //
                        // Soft delete account by updating its fields
                        //
                        let _ = diesel::update(
                            account_model::table.find(account_id),
                        )
                        .set((
                            account_model::name
                                .eq(account_id_string.to_owned()),
                            account_model::slug.eq(account_id_string),
                            account_model::is_active.eq(false),
                            account_model::updated
                                .eq(Some(Local::now().naive_utc())),
                            account_model::meta.eq(serde_json::to_value(
                                HashMap::<String, String>::new(),
                            )
                            .unwrap()),
                        ))
                        .execute(conn)
                        .map_err(InternalError::from);

                        //
                        // Remove all associated tags
                        //
                        let _ = diesel::delete(account_tag_model::table)
                            .filter(
                                account_tag_model::account_id.eq(account_id),
                            )
                            .execute(conn)
                            .map_err(InternalError::from);

                        //
                        // Remove all associated guest users
                        //
                        let _ =
                            diesel::delete(guest_user_on_account_model::table)
                                .filter(
                                    guest_user_on_account_model::account_id
                                        .eq(account_id),
                                )
                                .execute(conn)
                                .map_err(InternalError::from);

                        //
                        // Remove all associated manager accounts on tenant
                        //
                        let _ = diesel::delete(
                            manager_account_on_tenant_model::table,
                        )
                        .filter(
                            manager_account_on_tenant_model::account_id
                                .eq(account_id),
                        )
                        .execute(conn)
                        .map_err(InternalError::from);

                        Ok::<(), InternalError>(())
                    });

                match transaction_result {
                    Ok(_) => Ok(DeletionResponseKind::Deleted),
                    Err(InternalError::Database(e)) => {
                        deletion_err(format!("Database error: {e}")).as_error()
                    }
                    _ => {
                        deletion_err("Failed to soft delete account").as_error()
                    }
                }
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                account_id,
                "Account not found".to_string(),
            )),
        }
    }

    #[tracing::instrument(name = "hard_delete_account", skip_all)]
    async fn hard_delete(
        &self,
        account_id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = account_model::table.into_boxed();

        // Apply related accounts filter if provided
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            let ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
            query = query.filter(
                account_model::id.eq_any(
                    ids.iter()
                        .map(|id| Uuid::from_str(id).unwrap())
                        .collect::<Vec<_>>(),
                ),
            );
        }

        // Check if account exists and is allowed
        let account_exists = query
            .filter(account_model::id.eq(account_id))
            .select(account_model::id)
            .first::<Uuid>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check account: {}", e))
            })?;

        match account_exists {
            Some(_) => {
                // Delete account
                diesel::delete(account_model::table.find(account_id))
                    .execute(conn)
                    .map_err(|e| {
                        deletion_err(format!(
                            "Failed to hard delete account: {e}"
                        ))
                    })?;

                Ok(DeletionResponseKind::Deleted)
            }
            None => Ok(DeletionResponseKind::NotDeleted(
                account_id,
                "Account not found".to_string(),
            )),
        }
    }

    #[tracing::instrument(name = "delete_account_meta", skip_all)]
    async fn delete_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let transaction_result = conn.transaction(|conn| {
            // Get current account and its meta
            let account = account_model::table
                .find(account_id)
                .select(account_model::meta)
                .first::<Option<JsonValue>>(conn)?;

            let mut meta_map: HashMap<String, String> = match account {
                Some(meta) => serde_json::from_value(meta).unwrap_or_default(),
                None => HashMap::new(),
            };

            // Remove key if exists
            meta_map.remove(&key.to_string());

            // Update account meta
            diesel::update(account_model::table)
                .filter(account_model::id.eq(account_id))
                .set(
                    account_model::meta
                        .eq(serde_json::to_value(meta_map).unwrap()),
                )
                .execute(conn)?;

            Ok::<(), diesel::result::Error>(())
        });

        match transaction_result {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(e) => {
                deletion_err(format!("Failed to delete account meta: {}", e))
                    .as_error()
            }
        }
    }
}
