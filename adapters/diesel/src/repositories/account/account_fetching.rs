use super::shared::map_account_model_to_dto;
use crate::{
    models::{account::Account as AccountModel, config::DbPoolProvider},
    schema::{
        account::{self as account_model, dsl as account_dsl},
        account_tag::dsl as account_tag_dsl,
        user::{self as user_model, dsl as user_dsl},
    },
};

use async_trait::async_trait;
use diesel::{prelude::*, dsl::sql};
use myc_core::domain::{
    dtos::{
        account::Account, account_type::AccountType,
        native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts,
    },
    entities::AccountFetching,
};
use mycelium_base::{
    dtos::PaginatedRecord,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountFetching)]
pub struct AccountFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl AccountFetching for AccountFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_account", skip_all)]
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
            query = query.filter(account_model::id.eq_any(
                ids.iter().map(|id| id.to_string()).collect::<Vec<String>>(),
            ));
        }

        // Fetch account and its relationships
        let account = query
            .filter(account_model::id.eq(id.to_string()))
            .left_join(user_model::table)
            .select(AccountModel::as_select())
            .first::<AccountModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch account: {}", e))
            })?;

        match account {
            Some(record) => {
                Ok(FetchResponseKind::Found(map_account_model_to_dto(record)))
            }
            None => Ok(FetchResponseKind::NotFound(Some(id))),
        }
    }

    #[tracing::instrument(name = "list_accounts", skip_all)]
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

        let base_query =
            account_dsl::account.left_join(user_dsl::user).left_join(
                account_tag_dsl::account_tag
                    .on(account_dsl::id.eq(account_tag_dsl::account_id)),
            );

        let mut count_query = base_query.into_boxed();
        let mut records_query = base_query.into_boxed();

        if let Some(term_value) = term {
            let dsl = account_dsl::name.ilike(format!("%{}%", term_value));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(account_id_value) = account_id {
            let dsl = account_dsl::id.eq(account_id_value.to_string());
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(is_active) = is_account_active {
            let dsl = account_dsl::is_active.eq(is_active);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(is_checked) = is_account_checked {
            let dsl = account_dsl::is_checked.eq(is_checked);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(is_archived) = is_account_archived {
            let dsl = account_dsl::is_archived.eq(is_archived);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(tag_id_value) = tag_id {
            let dsl = account_tag_dsl::id.eq(tag_id_value.to_string());
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(tag_value_str) = tag_value {
            let dsl = account_tag_dsl::value.eq(tag_value_str);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(is_active) = is_owner_active {
            let dsl = user_dsl::is_active.eq(is_active);
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(acc_type) = account_type {
            let dsl = sql::<diesel::sql_types::Bool>(&format!(
                "account_type::jsonb @> '{}'",
                match serde_json::to_string(&acc_type) {
                    Ok(json) => json,
                    Err(e) => {
                        return creation_err(format!(
                            "Failed to serialize account type: {}",
                            e
                        ))
                        .as_error();
                    }
                }
            ));

            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            let dsl = account_dsl::id.eq_any(
                ids.iter().map(|id| id.to_string()).collect::<Vec<String>>(),
            );
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        let page_size = page_size.unwrap_or(10) as i64;

        let records = records_query
            .select(AccountModel::as_select())
            .order_by(account_dsl::created.desc())
            .limit(page_size)
            .offset(skip.unwrap_or(0) as i64)
            .load::<AccountModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch accounts: {}", e))
            })?;

        let total = count_query
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to count accounts: {}", e))
            })?;

        let records =
            records.into_iter().map(map_account_model_to_dto).collect();

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count: total,
            skip: Some(skip.unwrap_or(0) as i64),
            size: Some(page_size),
            records,
        }))
    }
}
