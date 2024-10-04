use crate::{
    prisma::{
        account as account_model, account_tags as account_tags_model,
        user as user_model, QueryMode,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountTypeV2,
        email::Email,
        native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts,
        tag::Tag,
        user::User,
    },
    entities::AccountFetching,
};
use mycelium_base::{
    dtos::{
        Children, {PaginatedRecord, Parent},
    },
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, fetching_err, MappedErrors},
};
use prisma_client_rust::{and, operator::and, or, Direction};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountFetching)]
pub struct AccountFetchingSqlDbRepository {}

#[async_trait]
impl AccountFetching for AccountFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get the account
        // ? -------------------------------------------------------------------

        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            if !ids.contains(&id) {
                return Ok(FetchResponseKind::NotFound(Some(id)));
            }
        };

        match client
            .account()
            .find_unique(account_model::id::equals(id.to_owned().to_string()))
            .include(account_model::include!({
                owners
                tags: select {
                    id
                    value
                    meta
                }
            }))
            .exec()
            .await
        {
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err,
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => {
                    let id = Uuid::from_str(&record.id).unwrap();

                    Ok(FetchResponseKind::Found(Account {
                        id: Some(id),
                        name: record.name,
                        slug: record.slug,
                        tags: match record.tags.len() {
                            0 => None,
                            _ => Some(
                                record
                                    .tags
                                    .to_owned()
                                    .into_iter()
                                    .map(|i| Tag {
                                        id: Uuid::parse_str(&i.id).unwrap(),
                                        value: i.value,
                                        meta: match i.meta {
                                            None => None,
                                            Some(meta) => {
                                                Some(from_value(meta).unwrap())
                                            }
                                        },
                                    })
                                    .collect::<Vec<Tag>>(),
                            ),
                        },
                        is_active: record.is_active,
                        is_checked: record.is_checked,
                        is_archived: record.is_archived,
                        verbose_status: Some(VerboseStatus::from_flags(
                            record.is_active,
                            record.is_checked,
                            record.is_archived,
                        )),
                        is_default: record.is_default,
                        owners: Children::Records(
                            record
                                .owners
                                .into_iter()
                                .map(|owner| {
                                    User::new(
                                        Some(
                                            Uuid::parse_str(&owner.id).unwrap(),
                                        ),
                                        owner.username,
                                        Email::from_string(owner.email)
                                            .unwrap(),
                                        Some(owner.first_name),
                                        Some(owner.last_name),
                                        owner.is_active,
                                        owner.created.into(),
                                        match owner.updated {
                                            None => None,
                                            Some(date) => {
                                                Some(date.with_timezone(&Local))
                                            }
                                        },
                                        Some(Parent::Id(id)),
                                        None,
                                    )
                                    .with_principal(owner.is_principal)
                                })
                                .collect::<Vec<User>>(),
                        ),
                        account_type: from_value(record.account_type).unwrap(),
                        guest_users: None,
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(res) => Some(DateTime::from(res)),
                        },
                    }))
                }
            },
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
        account_type: Option<AccountTypeV2>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? BUild query statement
        // ? -------------------------------------------------------------------

        let page_size = page_size.unwrap_or(10);
        let skip = skip.unwrap_or(0);
        let mut query_stmt = vec![];
        let mut and_query_stmt = vec![];

        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            and_query_stmt.push(account_model::id::in_vec(
                ids.into_iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>(),
            ))
        }

        if let Some(account_type) = account_type {
            and_query_stmt.push(account_model::account_type::equals(
                to_value(account_type).unwrap(),
            ))
        }

        if let Some(term) = term {
            query_stmt.push(or![
                and![
                    account_model::name::mode(QueryMode::Insensitive),
                    account_model::name::contains(term.to_owned()),
                ],
                account_model::owners::some(vec![and![
                    user_model::email::mode(QueryMode::Insensitive),
                    user_model::email::contains(term),
                ]])
            ]);
        }

        if let Some(account_id) = account_id {
            query_stmt.push(account_model::id::equals(account_id.to_string()));
        }

        if let Some(is_owner_active) = is_owner_active {
            and_query_stmt.push(account_model::owners::some(vec![
                user_model::is_active::equals(is_owner_active),
            ]));
        }

        if let Some(is_account_active) = is_account_active {
            and_query_stmt
                .push(account_model::is_active::equals(is_account_active));
        }

        if let Some(is_account_checked) = is_account_checked {
            and_query_stmt
                .push(account_model::is_checked::equals(is_account_checked));
        }

        if let Some(is_account_archived) = is_account_archived {
            and_query_stmt
                .push(account_model::is_archived::equals(is_account_archived));
        }

        if let Some(tag_value) = tag_value {
            and_query_stmt.push(account_model::tags::some(vec![and![
                account_tags_model::value::mode(QueryMode::Insensitive),
                account_tags_model::value::contains(tag_value),
            ]]));
        }

        if let Some(tag_id) = tag_id {
            and_query_stmt.push(account_model::tags::some(vec![
                account_tags_model::id::equals(tag_id.to_string()),
            ]));
        }

        if !and_query_stmt.is_empty() {
            query_stmt.push(and(and_query_stmt));
        }

        // ? -------------------------------------------------------------------
        // ? List accounts
        // ? -------------------------------------------------------------------

        let (count, response) = match client
            ._batch((
                client.account().count(query_stmt.to_owned()),
                client
                    .account()
                    .find_many(query_stmt)
                    .skip(skip.into())
                    .take(page_size.into())
                    .order_by(account_model::updated::order(Direction::Desc))
                    .include(account_model::include!({
                        owners
                        tags: select {
                            id
                            value
                            meta
                        }
                    })),
            ))
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on fetch accounts: {err}",
                ))
                .as_error()
            }
            Ok(res) => res,
        };

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let records: Vec<Account> = response
            .into_iter()
            .map(|record| {
                let id = Uuid::from_str(&record.id).unwrap();

                Account {
                    id: Some(id),
                    name: record.name,
                    slug: record.slug,
                    tags: match record.tags.len() {
                        0 => None,
                        _ => Some(
                            record
                                .tags
                                .to_owned()
                                .into_iter()
                                .map(|i| Tag {
                                    id: Uuid::parse_str(&i.id).unwrap(),
                                    value: i.value,
                                    meta: match i.meta {
                                        None => None,
                                        Some(meta) => {
                                            Some(from_value(meta).unwrap())
                                        }
                                    },
                                })
                                .collect::<Vec<Tag>>(),
                        ),
                    },
                    is_active: record.is_active,
                    is_checked: record.is_checked,
                    is_archived: record.is_archived,
                    verbose_status: Some(VerboseStatus::from_flags(
                        record.is_active,
                        record.is_checked,
                        record.is_archived,
                    )),
                    is_default: record.is_default,
                    owners: Children::Records(
                        record
                            .owners
                            .into_iter()
                            .map(|owner| {
                                User::new(
                                    Some(Uuid::parse_str(&owner.id).unwrap()),
                                    owner.username,
                                    Email::from_string(owner.email).unwrap(),
                                    Some(owner.first_name),
                                    Some(owner.last_name),
                                    owner.is_active,
                                    owner.created.into(),
                                    match owner.updated {
                                        None => None,
                                        Some(date) => {
                                            Some(date.with_timezone(&Local))
                                        }
                                    },
                                    Some(Parent::Id(id)),
                                    None,
                                )
                                .with_principal(owner.is_principal)
                            })
                            .collect::<Vec<User>>(),
                    ),
                    account_type: from_value(record.account_type).unwrap(),
                    guest_users: None,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                }
            })
            .collect::<Vec<Account>>();

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count,
            skip: Some(skip.into()),
            size: Some(page_size.into()),
            records,
        }))
    }
}
