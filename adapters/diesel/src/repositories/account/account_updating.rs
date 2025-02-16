use crate::{
    models::{account::Account as AccountModel, config::DbPoolProvider},
    schema::account as account_model,
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        account::{Account, AccountMeta, AccountMetaKey, VerboseStatus},
        account_type::AccountType,
        native_error_codes::NativeErrorCodes,
    },
    entities::AccountUpdating,
};
use mycelium_base::{
    dtos::Children,
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use serde_json::{from_value, to_value, Value as JsonValue};
use shaku::Component;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountUpdating)]
pub struct AccountUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl AccountUpdating for AccountUpdatingSqlDbRepository {
    #[tracing::instrument(name = "update_account", skip_all)]
    async fn update(
        &self,
        account: Account,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let account_id = match account.id {
            Some(id) => id,
            None => {
                return updating_err("Account ID is required for update")
                    .with_code(NativeErrorCodes::MYC00001)
                    .as_error()
            }
        };

        let updated = diesel::update(account_model::table.find(account_id))
            .set((
                account_model::name.eq(account.name),
                account_model::slug.eq(account.slug),
                account_model::is_active.eq(account.is_active),
                account_model::is_checked.eq(account.is_checked),
                account_model::is_archived.eq(account.is_archived),
                account_model::updated.eq(Some(Local::now().naive_utc())),
            ))
            .returning(AccountModel::as_returning())
            .get_result(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update account: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(
            self.map_account_model_to_dto(updated),
        ))
    }

    #[tracing::instrument(name = "update_own_account_name", skip_all)]
    async fn update_own_account_name(
        &self,
        account_id: Uuid,
        name: String,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated = diesel::update(account_model::table.find(account_id))
            .set((
                account_model::name.eq(name),
                account_model::updated.eq(Some(Local::now().naive_utc())),
            ))
            .returning(AccountModel::as_returning())
            .get_result(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update account name: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(
            self.map_account_model_to_dto(updated),
        ))
    }

    #[tracing::instrument(name = "update_account_type", skip_all)]
    async fn update_account_type(
        &self,
        account_id: Uuid,
        account_type: AccountType,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let updated = diesel::update(account_model::table.find(account_id))
            .set((
                account_model::account_type.eq(to_value(account_type).unwrap()),
                account_model::updated.eq(Some(Local::now().naive_utc())),
            ))
            .returning(AccountModel::as_returning())
            .get_result(conn)
            .map_err(|e| {
                updating_err(format!("Failed to update account type: {}", e))
            })?;

        Ok(UpdatingResponseKind::Updated(
            self.map_account_model_to_dto(updated),
        ))
    }

    #[tracing::instrument(name = "update_account_meta", skip_all)]
    async fn update_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
        value: String,
    ) -> Result<UpdatingResponseKind<AccountMeta>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
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

            // Update or insert new meta value
            meta_map.insert(key.to_string(), value);

            // Update account meta
            diesel::update(account_model::table)
                .filter(account_model::id.eq(account_id))
                .set(account_model::meta.eq(to_value(&meta_map).unwrap()))
                .execute(conn)?;

            Ok::<AccountMeta, diesel::result::Error>(AccountMeta::from_iter(
                meta_map.iter().map(|(k, v)| {
                    (AccountMetaKey::from_str(k).unwrap(), v.to_string())
                }),
            ))
        });

        match transaction_result {
            Ok(meta) => Ok(UpdatingResponseKind::Updated(meta)),
            Err(e) => {
                updating_err(format!("Failed to update account meta: {}", e))
                    .as_error()
            }
        }
    }
}

impl AccountUpdatingSqlDbRepository {
    fn map_account_model_to_dto(&self, model: AccountModel) -> Account {
        Account {
            id: Some(model.id),
            name: model.name,
            slug: model.slug,
            tags: None,
            is_active: model.is_active,
            is_checked: model.is_checked,
            is_archived: model.is_archived,
            verbose_status: Some(VerboseStatus::from_flags(
                model.is_active,
                model.is_checked,
                model.is_archived,
            )),
            is_default: model.is_default,
            owners: Children::Records(vec![]),
            account_type: from_value(model.account_type).unwrap(),
            guest_users: None,
            created: model.created.and_local_timezone(Local).unwrap(),
            updated: model
                .updated
                .map(|dt| dt.and_local_timezone(Local).unwrap()),
            meta: None,
        }
    }
}
