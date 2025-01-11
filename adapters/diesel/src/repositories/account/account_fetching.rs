use crate::{
    models::{
        account::Account as AccountModel, config::DbConfig,
        user::User as UserModel,
    },
    schema::{account as account_model, user as user_model},
};

use async_trait::async_trait;
use chrono::Local;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountType,
        email::Email,
        native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts,
        user::User,
    },
    entities::AccountFetching,
};
use mycelium_base::{
    dtos::{Children, PaginatedRecord, Parent},
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountFetching)]
pub struct AccountFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl AccountFetching for AccountFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = account_model::table.into_boxed();

        // Apply related accounts filter if provided
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            query = query.filter(account_model::id.eq_any(ids));
        }

        // Fetch account and its relationships
        let account = query
            .filter(account_model::id.eq(id))
            .left_join(user_model::table)
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch account: {}", e))
            })?;

        match account {
            Some(record) => {
                let record_id = record.id;
                // Fetch account owners
                let owners = user_model::table
                    .filter(user_model::account_id.eq(record_id))
                    .select(UserModel::as_select())
                    .load::<UserModel>(conn)
                    .map_err(|e| {
                        fetching_err(format!(
                            "Failed to fetch account owners: {}",
                            e
                        ))
                    })?;

                // Convert model to DTO
                let mut account_dto =
                    self.map_account_model_to_dto(record.to_owned());

                // Add owners to DTO
                account_dto.owners = Children::Records(
                    owners
                        .into_iter()
                        .map(|owner| {
                            User::new(
                                Some(owner.id),
                                owner.username,
                                Email::from_string(owner.email).unwrap(),
                                Some(owner.first_name),
                                Some(owner.last_name),
                                owner.is_active,
                                owner
                                    .created
                                    .and_local_timezone(Local)
                                    .unwrap(),
                                owner.updated.map(|dt| {
                                    dt.and_local_timezone(Local).unwrap()
                                }),
                                Some(Parent::Id(record.id)),
                                None,
                            )
                            .with_principal(owner.is_principal)
                        })
                        .collect(),
                );

                Ok(FetchResponseKind::Found(account_dto))
            }
            None => Ok(FetchResponseKind::NotFound(Some(id))),
        }
    }

    async fn list(
        &self,
        related_accounts: RelatedAccounts,
        term: Option<String>,
        is_owner_active: Option<bool>,
        is_account_active: Option<bool>,
        is_account_checked: Option<bool>,
        is_account_archived: Option<bool>,
        tag_id: Option<Uuid>,
        tag_value: Option<String>,
        account_id: Option<Uuid>,
        account_type: Option<AccountType>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = account_model::table
            .left_join(user_model::table)
            .into_boxed();

        // Apply filters
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            query = query.filter(account_model::id.eq_any(ids));
        }

        if let Some(term) = term {
            query = query.filter(
                account_model::name
                    .ilike(format!("%{}%", term))
                    .or(account_model::slug.ilike(format!("%{}%", term))),
            );
        }

        if let Some(account_type) = account_type {
            query = query.filter(
                account_model::account_type
                    .eq(serde_json::to_value(account_type).unwrap()),
            );
        }

        if let Some(account_id) = account_id {
            query = query.filter(account_model::id.eq(account_id));
        }

        if let Some(is_account_active) = is_account_active {
            query =
                query.filter(account_model::is_active.eq(is_account_active));
        }

        if let Some(is_account_checked) = is_account_checked {
            query =
                query.filter(account_model::is_checked.eq(is_account_checked));
        }

        if let Some(is_account_archived) = is_account_archived {
            query = query
                .filter(account_model::is_archived.eq(is_account_archived));
        }

        // Adicionar filtro de is_owner_active
        if let Some(is_owner_active) = is_owner_active {
            query = query.filter(user_model::is_active.eq(is_owner_active));
        }

        // Adicionar filtros de tags
        if let Some(tag_id) = tag_id {
            let tag_json = if let Some(tag_value) = tag_value {
                serde_json::json!([{"id": tag_id, "value": tag_value}])
            } else {
                serde_json::json!([{"id": tag_id}])
            };
            query = query.filter(account_model::tags.contains(tag_json));
        }

        // Get total count
        let total = account_model::table
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to get total count: {}", e))
            })?;

        // Apply pagination
        let page_size = i64::from(page_size.unwrap_or(10));
        let skip = i64::from(skip.unwrap_or(0));

        let records = query
            .offset(skip)
            .limit(page_size)
            .select(AccountModel::as_select())
            .load::<AccountModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch accounts: {}", e))
            })?;

        // Convert models to DTOs
        let accounts = records
            .into_iter()
            .map(|record| self.map_account_model_to_dto(record))
            .collect();

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count: total as i64,
            skip: Some(skip as i64),
            size: Some(page_size as i64),
            records: accounts,
        }))
    }
}

impl AccountFetchingSqlDbRepository {
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
