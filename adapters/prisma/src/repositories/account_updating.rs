use crate::{
    prisma::account as account_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountTypeV2,
        email::Email,
        native_error_codes::NativeErrorCodes,
        tag::Tag,
        user::User,
    },
    entities::AccountUpdating,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use serde_json::from_value;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountUpdating)]
pub struct AccountUpdatingSqlDbRepository {}

#[async_trait]
impl AccountUpdating for AccountUpdatingSqlDbRepository {
    async fn update(
        &self,
        account: Account,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------

        let account_id = match account.id {
            None => {
                return updating_err(String::from(
                    "Unable to update account. Invalid record ID",
                ))
                .with_exp_true()
                .as_error()
            }
            Some(res) => res,
        };

        unimplemented!("Need to implement the update method");

        /* let response = client
            .account()
            .update(
                account_model::id::equals(account_id.to_string()),
                vec![
                    account_model::name::set(account.name),
                    account_model::slug::set(account.slug),
                    account_model::is_active::set(account.is_active),
                    account_model::is_checked::set(account.is_checked),
                    account_model::is_archived::set(account.is_archived),
                    account_model::is_default::set(account.is_default),
                    account_model::account_type_id::set(match account.account_type {
                        Parent::Id(id) => id.to_string(),
                        Parent::Record(record) => match record.id {
                            None => {
                                return updating_err(
                                    String::from("Unable to update account. Invalid account type ID"),
                                ).with_exp_true().as_error()
                            }
                            Some(id) => id.to_string(),
                        }
                    })
                ],
            )
            .include(account_model::include!({
                owners
                tags: select {
                    id
                    value
                    meta
                }
            }))
            .exec()
            .await;

        match response {
            Ok(record) => {
                let id = Uuid::parse_str(&record.id).unwrap();

                Ok(UpdatingResponseKind::Updated(Account {
                    id: Some(id.to_owned()),
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
                    account_type: Parent::Id(
                        Uuid::from_str(&record.account_type_id).unwrap(),
                    ),
                    guest_users: None,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                }))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        account_id
                    ))
                    .with_exp_true()
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        } */
    }

    async fn update_own_account_name(
        &self,
        account_id: Uuid,
        name: String,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        unimplemented!()
    }

    async fn update_account_type(
        &self,
        account_id: Uuid,
        account_type: AccountTypeV2,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        unimplemented!()
    }
}
