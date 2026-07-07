use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::user::User as UserModel,
    repositories::internal_error::InternalError,
    schema::{
        account as account_model, account_tag as account_tag_model,
        guest_user_on_account as guest_user_on_account_model,
        manager_account_on_tenant as manager_account_on_tenant_model,
        user as user_model,
    },
    types::{naive_timestamp_to_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        account::AccountMetaKey, account_type::AccountType,
        native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts,
    },
    entities::AccountDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountDeletion)]
pub struct AccountDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl AccountDeletion for AccountDeletionSqlDbRepository {
    #[tracing::instrument(name = "soft_delete_account", skip_all)]
    async fn soft_delete_account(
        &self,
        account_id: Uuid,
        account_type: AccountType,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let account_id_text = uuid_to_text(&account_id);

        let account_type_json =
            serde_json::to_string(&account_type).map_err(|e| {
                deletion_err(format!("Failed to serialize account type: {e}"))
            })?;

        let mut query = account_model::table.into_boxed();

        // Apply related accounts filter if provided
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            let ids: Vec<String> = ids.iter().map(uuid_to_text).collect();
            query = query.filter(account_model::id.eq_any(ids));
        }

        // Check if account exists and is allowed
        let account_exists = query
            .filter(
                account_model::id
                    .eq(&account_id_text)
                    .and(account_model::account_type.eq(&account_type_json)),
            )
            .select(account_model::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check account: {}", e))
            })?;

        let Some(_) = account_exists else {
            return Ok(DeletionResponseKind::NotDeleted(
                account_id,
                "Account not found".to_string(),
            ));
        };

        let account_id_string = format!("{}-deleted", account_id);

        let transaction_result: Result<(), _> = conn.transaction(|conn| {
            //
            // Soft delete account by updating its fields
            //
            let _ =
                diesel::update(account_model::table.find(&account_id_text))
                    .set((
                        account_model::name.eq(account_id_string.to_owned()),
                        account_model::slug.eq(account_id_string),
                        account_model::is_active.eq(false),
                        account_model::is_deleted.eq(true),
                        account_model::updated.eq(Some(
                            naive_timestamp_to_text(&Local::now().naive_utc()),
                        )),
                        account_model::meta.eq(serde_json::to_string(
                            &HashMap::<String, String>::new(),
                        )
                        .unwrap()),
                    ))
                    .execute(conn)
                    .map_err(InternalError::from);

            let optional_user = user_model::table
                .filter(user_model::account_id.eq(&account_id_text))
                .select(UserModel::as_select())
                .first::<UserModel>(conn)
                .optional()
                .map_err(InternalError::from)?;

            if let Some(user) = optional_user {
                let user_id = format!("{}-deleted", user.id);

                let _ = diesel::update(
                    user_model::table
                        .filter(user_model::account_id.eq(&account_id_text)),
                )
                .set((
                    user_model::username.eq(user_id.to_owned()),
                    user_model::email.eq(user_id.to_owned()),
                    user_model::first_name.eq(""),
                    user_model::last_name.eq(""),
                    user_model::is_active.eq(false),
                    user_model::is_principal.eq(false),
                    user_model::updated.eq(Some(naive_timestamp_to_text(
                        &Local::now().naive_utc(),
                    ))),
                ))
                .execute(conn)
                .map_err(InternalError::from);
            }

            //
            // Remove all associated tags
            //
            let _ = diesel::delete(account_tag_model::table)
                .filter(account_tag_model::account_id.eq(&account_id_text))
                .execute(conn)
                .map_err(InternalError::from);

            //
            // Remove all associated guest users
            //
            let _ = diesel::delete(guest_user_on_account_model::table)
                .filter(
                    guest_user_on_account_model::account_id
                        .eq(&account_id_text),
                )
                .execute(conn)
                .map_err(InternalError::from);

            //
            // Remove all associated manager accounts on tenant
            //
            let _ = diesel::delete(manager_account_on_tenant_model::table)
                .filter(
                    manager_account_on_tenant_model::account_id
                        .eq(&account_id_text),
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
            _ => deletion_err("Failed to soft delete account").as_error(),
        }
    }

    #[tracing::instrument(name = "hard_delete_account", skip_all)]
    async fn hard_delete_account(
        &self,
        account_id: Uuid,
        account_type: AccountType,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let account_id_text = uuid_to_text(&account_id);

        let account_type_json =
            serde_json::to_string(&account_type).map_err(|e| {
                deletion_err(format!("Failed to serialize account type: {e}"))
            })?;

        let mut query = account_model::table.into_boxed();

        // Apply related accounts filter if provided
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            let ids: Vec<String> = ids.iter().map(uuid_to_text).collect();
            query = query.filter(account_model::id.eq_any(ids));
        }

        // Check if account exists and is allowed
        let account_exists = query
            .filter(
                account_model::id
                    .eq(&account_id_text)
                    .and(account_model::account_type.eq(&account_type_json)),
            )
            .select(account_model::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                deletion_err(format!("Failed to check account: {}", e))
            })?;

        match account_exists {
            Some(_) => {
                // Delete account
                diesel::delete(account_model::table.find(&account_id_text))
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

        let account_id_text = uuid_to_text(&account_id);

        let transaction_result = conn.transaction(|conn| {
            // Get current account and its meta
            let meta = account_model::table
                .find(&account_id_text)
                .select(account_model::meta)
                .first::<Option<String>>(conn)?;

            let mut meta_map: HashMap<String, String> = match meta {
                Some(meta) => serde_json::from_str(&meta).unwrap_or_default(),
                None => HashMap::new(),
            };

            // Remove key if exists
            meta_map.remove(&key.to_string());

            // Update account meta
            diesel::update(account_model::table)
                .filter(account_model::id.eq(&account_id_text))
                .set(
                    account_model::meta
                        .eq(serde_json::to_string(&meta_map).unwrap()),
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
