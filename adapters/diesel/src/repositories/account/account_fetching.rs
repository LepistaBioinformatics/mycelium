use super::shared::map_account_model_to_dto;
use crate::{
    models::{account::Account as AccountModel, config::DbConfig},
    schema::{account as account_model, user as user_model},
};

use async_trait::async_trait;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Json};
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

fn option_to_null<T: ToString>(opt: Option<T>) -> String {
    match opt {
        Some(val) => val.to_string(),
        None => "NULL".to_string(),
    }
}

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
                Ok(FetchResponseKind::Found(map_account_model_to_dto(record)))
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

        let mut sql = "".to_string();

        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            //
            // Related accounts filter the account id to check if the operation is permitted
            //
            sql.push_str(&format!(
                " AND a.id IN ({})",
                ids.iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ));
        }

        // Calculating the offset
        let offset = (skip.unwrap_or(0) * page_size.unwrap_or(10)) as i64;

        // Build the SQL query with WITH
        sql.push_str(&format!(
            r#"
            WITH paginaged AS (
                SELECT 
                    a.id,
                    a.name,
                    a.is_active,
                    a.is_checked,
                    a.is_archived,
                    a.tags,
                    oa.owner_id,
                    oa.is_active,
                    at.tag_id,
                    at.value
                FROM account a
                LEFT JOIN owner_on_account oa ON a.id = oa.account_id
                LEFT JOIN account_tag at ON a.id = at.account_id
                WHERE (
                    {term} IS NULL OR a.name ILIKE {term}
                    AND {account_id} IS NULL OR a.id = {account_id}
                    AND {is_account_active} IS NULL OR a.is_active = {is_account_active}
                    AND {is_account_checked} IS NULL OR a.is_checked = {is_account_checked}
                    AND {is_account_archived} IS NULL OR a.is_archived = {is_account_archived}
                    AND {tag_id} IS NULL OR at.tag_id = {tag_id}
                    AND {tag_value} IS NULL OR at.value = {tag_value}
                    AND {is_owner_active} IS NULL OR oa.is_active = {is_owner_active}
                    AND {account_type} IS NULL OR a.account_type = {account_type}
                )
                ORDER BY a.created DESC
                LIMIT {page_size} OFFSET {offset}
            )
            SELECT
                (SELECT COUNT(*) FROM paginaged) as total,
                ARRAY(SELECT * FROM paginaged) as records
            "#,
            term = option_to_null(term),
            account_id = option_to_null(account_id),
            is_account_active = option_to_null(is_account_active),
            is_account_checked = option_to_null(is_account_checked),
            is_account_archived = option_to_null(is_account_archived),
            tag_id = option_to_null(tag_id),
            tag_value = option_to_null(tag_value),
            is_owner_active = option_to_null(is_owner_active),
            account_type = match account_type {
                Some(account_type) => account_type.to_string(),
                None => "NULL".to_string(),
            },
            page_size = page_size.unwrap_or(10),
            offset = offset,
        ));

        println!("{}", sql);

        let results: Vec<AccountWithCount> = diesel::sql_query(&sql)
            .bind::<BigInt, _>(page_size.unwrap_or(10) as i64)
            .bind::<BigInt, _>(offset)
            .get_results(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to execute query: {}", e))
            })?;

        let total = if results.is_empty() {
            0
        } else {
            results[0].total
        };

        let records = if let serde_json::Value::Array(arr) = &results[0].records
        {
            arr.iter()
                .map(|r| serde_json::from_value(r.clone()).unwrap())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count: total,
            skip: Some(skip.unwrap_or(0) as i64),
            size: Some(page_size.unwrap_or(10) as i64),
            records,
        }))
    }
}

#[derive(QueryableByName, Debug)]
#[diesel(table_name = account_model)]
#[diesel(check_for_backend(Pg))]
struct AccountWithCount {
    #[diesel(sql_type = BigInt)]
    total: i64,
    #[diesel(sql_type = Json)]
    records: serde_json::Value,
}
