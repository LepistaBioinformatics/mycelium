use super::shared::map_account_model_to_dto;
use crate::{
    models::{
        account::Account as AccountModel,
        account_tag::AccountTag as AccountTagModel, config::DbPoolProvider,
        user::User as UserModel,
    },
    schema::{
        account::{self as account_model, dsl as account_dsl},
        account_tag::dsl as account_tag_dsl,
        user::{self as user_model, dsl as user_dsl},
    },
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{dsl::sql, prelude::*};
use myc_core::domain::{
    dtos::{
        account::Account, account_type::AccountType, email::Email,
        native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts, tag::Tag, user::User,
    },
    entities::AccountFetching,
};
use mycelium_base::{
    dtos::Children,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, fetching_err, MappedErrors},
};
use shaku::Component;
use std::{str::FromStr, sync::Arc};
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
            let ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();

            query = query.filter(
                account_model::id.eq_any(
                    ids.iter()
                        .map(|id| Uuid::from_str(id).unwrap())
                        .collect::<Vec<_>>(),
                ),
            );
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

        if let Some(account) = account {
            let tags = AccountTagModel::belonging_to(&account)
                .select(AccountTagModel::as_select())
                .load::<AccountTagModel>(conn)
                .map_err(|e| {
                    fetching_err(format!("Failed to fetch tags: {}", e))
                })?
                .into_iter()
                .map(|t| Tag {
                    id: t.id,
                    value: t.value,
                    meta: t.meta.map(|m| serde_json::from_value(m).unwrap()),
                })
                .collect::<Vec<Tag>>();

            let owners = UserModel::belonging_to(&account)
                .select(UserModel::as_select())
                .load::<UserModel>(conn)
                .map_err(|e| {
                    fetching_err(format!("Failed to fetch users: {}", e))
                })?
                .into_iter()
                .map(|o| {
                    User::new_public_redacted(
                        o.id,
                        Email::from_string(o.email).unwrap(),
                        o.username,
                        o.created.and_local_timezone(Local).unwrap(),
                        o.is_active,
                        o.is_principal,
                    )
                })
                .collect::<Vec<User>>();

            let mut account = map_account_model_to_dto(account);

            account.tags = match tags.len() {
                0 => None,
                _ => Some(tags),
            };

            if owners.len() > 0 {
                account.owners = Children::Records(owners);
            }

            return Ok(FetchResponseKind::Found(account));
        }

        Ok(FetchResponseKind::NotFound(Some(id)))
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
        account_type: AccountType,
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

        let account_type_dsl = sql::<diesel::sql_types::Bool>(&format!(
            "account_type::jsonb @> '{}'",
            match serde_json::to_string(&account_type) {
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

        let mut count_query =
            base_query.filter(account_type_dsl.clone()).into_boxed();

        let mut records_query =
            base_query.filter(account_type_dsl).into_boxed();

        if let Some(term_value) = term {
            let dsl = account_dsl::name.ilike(format!("%{}%", term_value));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(account_id_value) = account_id {
            let dsl = account_dsl::id.eq(account_id_value);
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
            let dsl = account_tag_dsl::id.eq(tag_id_value);
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

        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            let ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();

            let dsl = account_dsl::id.eq_any(
                ids.iter()
                    .map(|id| Uuid::from_str(id).unwrap())
                    .collect::<Vec<_>>(),
            );
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let records = records_query
            .select(AccountModel::as_select())
            .order_by(account_dsl::created.desc())
            .limit(page_size)
            .offset(skip)
            .load::<AccountModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch accounts: {}", e))
            })?;

        if records.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let tags = AccountTagModel::belonging_to(&records)
            .select(AccountTagModel::as_select())
            .load::<AccountTagModel>(conn)
            .map_err(|e| fetching_err(format!("Failed to fetch tags: {}", e)))?
            .grouped_by(&records);

        let owners = UserModel::belonging_to(&records)
            .select(UserModel::as_select())
            .load::<UserModel>(conn)
            .map_err(|e| fetching_err(format!("Failed to fetch users: {}", e)))?
            .grouped_by(&records);

        let total = count_query
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to count accounts: {}", e))
            })?;

        let accounts = records
            .into_iter()
            .zip(tags)
            .zip(owners)
            .map(|((account, tags), users)| {
                let mut account = map_account_model_to_dto(account);

                let tags = tags
                    .into_iter()
                    .map(|t| Tag {
                        id: t.id,
                        value: t.value,
                        meta: t
                            .meta
                            .map(|m| serde_json::from_value(m).unwrap()),
                    })
                    .collect::<Vec<Tag>>();

                let owners = users
                    .into_iter()
                    .map(|o| {
                        User::new_public_redacted(
                            o.id,
                            Email::from_string(o.email).unwrap(),
                            o.username,
                            o.created.and_local_timezone(Local).unwrap(),
                            o.is_active,
                            o.is_principal,
                        )
                    })
                    .collect::<Vec<User>>();

                account.tags = match tags.len() {
                    0 => None,
                    _ => Some(tags),
                };

                if owners.len() > 0 {
                    account.owners = Children::Records(owners);
                }

                account
            })
            .collect();

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip),
            size: Some(page_size),
            records: accounts,
        })
    }
}
