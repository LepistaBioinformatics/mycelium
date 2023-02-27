use crate::{
    prisma::{account as account_model, user as user_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    dtos::enums::{PaginatedRecord, ParentEnum},
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{
        account::{Account, AccountType},
        email::Email,
        user::User,
    },
    entities::AccountFetching,
};
use prisma_client_rust::{or, Direction};
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
    ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(creation_err(
                    String::from(
                        "Prisma Client error. Could not fetch client.",
                    ),
                    Some(false),
                    None,
                ))
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get the account
        // ? -------------------------------------------------------------------

        match client
            .account()
            .find_unique(account_model::id::equals(id.to_owned().to_string()))
            .include(account_model::include!({ account_type owner }))
            .exec()
            .await
        {
            Err(err) => {
                return Err(creation_err(
                    format!("Unexpected error on parse user email: {:?}", err,),
                    None,
                    None,
                ))
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => Ok(FetchResponseKind::Found(Account {
                    id: Some(Uuid::from_str(&record.id).unwrap()),
                    name: record.name,
                    is_active: record.is_active,
                    is_checked: record.is_checked,
                    is_archived: record.is_archived,
                    owner: ParentEnum::Record(User {
                        id: Some(Uuid::from_str(&record.owner.id).unwrap()),
                        username: record.owner.username,
                        email: Email::from_string(record.owner.email).unwrap(),
                        first_name: Some(record.owner.first_name),
                        last_name: Some(record.owner.last_name),
                        is_active: record.owner.is_active,
                        created: record.owner.created.into(),
                        updated: match record.owner.updated {
                            None => None,
                            Some(res) => Some(DateTime::from(res)),
                        },
                    }),
                    account_type: ParentEnum::Record(AccountType {
                        id: Some(
                            Uuid::from_str(&record.account_type.id).unwrap(),
                        ),
                        name: record.account_type.name,
                        description: record.account_type.description,
                        is_subscription: record.account_type.is_subscription,
                        is_manager: record.account_type.is_manager,
                        is_staff: record.account_type.is_staff,
                    }),
                    guest_users: None,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                })),
            },
        }
    }

    async fn list(
        &self,
        term: Option<String>,
        is_owner_active: Option<bool>,
        is_account_active: Option<bool>,
        is_account_checked: Option<bool>,
        is_account_archived: Option<bool>,
        account_type_id: Option<Uuid>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(creation_err(
                    String::from(
                        "Prisma Client error. Could not fetch client.",
                    ),
                    Some(false),
                    None,
                ))
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? BUild query statement
        // ? -------------------------------------------------------------------

        let page_size = page_size.unwrap_or(10);
        let skip = skip.unwrap_or(0);
        let mut query_stmt = vec![];

        if term.is_some() {
            let term = term.unwrap();
            query_stmt.push(or![
                account_model::name::contains(term.to_owned()),
                account_model::owner::is(vec![user_model::email::contains(
                    term
                )])
            ]);
        }

        if is_account_active.is_some() {
            query_stmt.push(account_model::owner::is(vec![
                user_model::is_active::equals(is_owner_active.unwrap()),
            ]));
        }

        if is_account_active.is_some() {
            query_stmt.push(account_model::is_active::equals(
                is_account_active.unwrap(),
            ));
        }

        if is_account_checked.is_some() {
            query_stmt.push(account_model::is_checked::equals(
                is_account_checked.unwrap(),
            ));
        }

        if is_account_archived.is_some() {
            query_stmt.push(account_model::is_checked::equals(
                is_account_archived.unwrap(),
            ));
        }

        if account_type_id.is_some() {
            query_stmt.push(account_model::account_type_id::equals(
                account_type_id.unwrap().to_string(),
            ));
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
                    .include(account_model::include!({ owner })),
            ))
            .await
        {
            Err(err) => {
                return Err(fetching_err(
                    format!("Unexpected error on parse user email: {:?}", err,),
                    None,
                    None,
                ))
            }
            Ok(res) => res,
        };

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let records: Vec<Account> = response
            .into_iter()
            .map(|record| Account {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                is_active: record.is_active,
                is_checked: record.is_checked,
                is_archived: record.is_archived,
                owner: ParentEnum::Record(User {
                    id: Some(Uuid::from_str(&record.owner_id).unwrap()),
                    username: record.owner.username,
                    email: Email::from_string(record.owner.email).unwrap(),
                    first_name: record.owner.first_name.into(),
                    last_name: record.owner.last_name.into(),
                    is_active: record.owner.is_active,
                    created: record.owner.created.into(),
                    updated: match record.owner.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                }),
                account_type: ParentEnum::Id(
                    Uuid::from_str(&record.account_type_id).unwrap(),
                ),
                guest_users: None,
                created: record.created.into(),
                updated: match record.updated {
                    None => None,
                    Some(res) => Some(DateTime::from(res)),
                },
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
