use crate::{models::config::DbPoolProvider, schema::account as account_model};

use async_trait::async_trait;
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
    #[tracing::instrument(name = "delete_account", skip_all)]
    async fn delete(
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
                        deletion_err(format!("Failed to delete account: {}", e))
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
