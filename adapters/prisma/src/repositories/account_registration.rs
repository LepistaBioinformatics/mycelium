use crate::{
    prisma::{
        account as account_model, account_type as account_type_model,
        user as user_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{
        account::{Account, AccountType, VerboseStatus},
        email::Email,
        user::User,
    },
    entities::AccountRegistration,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountRegistration)]
pub struct AccountRegistrationSqlDbRepository {}

#[async_trait]
impl AccountRegistration for AccountRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        account: Account,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code("MYC00001".to_string())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let response = client
            .account()
            .find_first(vec![
                account_model::name::equals(account.name.to_owned()),
                account_model::owner::is(vec![user_model::email::equals(
                    match account.owner.to_owned() {
                        ParentEnum::Id(_) => {
                            return creation_err(
                                String::from("Could not create account. User e-mail invalid."),
                                
                            ).as_error()
                        }
                        ParentEnum::Record(record) => {
                            record.email.get_email().to_owned()
                        }
                    },
                )]),
            ])
            .include(account_model::include!({ owner account_type }))
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                return Ok(GetOrCreateResponseKind::NotCreated(
                    Account {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        is_active: record.is_active,
                        is_checked: record.is_checked,
                        is_archived: record.is_archived,
                        verbose_status: Some(VerboseStatus::from_flags(
                            record.is_active,
                            record.is_checked,
                            record.is_archived,
                        )),
                        owner: ParentEnum::Record(User {
                            id: Some(
                                Uuid::parse_str(&record.owner.id).unwrap(),
                            ),
                            username: record.owner.username,
                            email: Email::from_string(record.owner.email)?,
                            first_name: Some(record.owner.first_name),
                            last_name: Some(record.owner.last_name),
                            is_active: record.owner.is_active,
                            created: record.owner.created.into(),
                            updated: match record.owner.updated {
                                None => None,
                                Some(date) => Some(date.with_timezone(&Local)),
                            },
                        }),
                        account_type: ParentEnum::Record(AccountType {
                            id: Some(
                                Uuid::parse_str(&record.account_type.id)
                                    .unwrap(),
                            ),
                            name: record.account_type.name,
                            description: record.account_type.description,
                            is_subscription: record
                                .account_type
                                .is_subscription,
                            is_manager: record.account_type.is_manager,
                            is_staff: record.account_type.is_staff,
                        }),
                        guest_users: None,
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                    },
                    "Account already exists".to_string(),
                ));
            }
            None => (),
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        let response = client
            .account()
            .create(
                account.name,
                user_model::id::equals(match account.owner {
                    ParentEnum::Id(_) => return creation_err(
                        String::from(
                            "Could not create account. Invalid owner.",
                        ),
                    ).as_error(),
                    ParentEnum::Record(record) => match record.id {
                        None => return creation_err(
                            String::from(
                                "Could not create account. User e-mail invalid.",
                            ),
                        ).as_error(),
                        Some(res) => res.to_string(),
                    }
                }),
                account_type_model::id::equals(match account.account_type {
                    ParentEnum::Id(_) => return creation_err(
                        String::from(
                            "Could not create account. Invalid account type.",
                        ),
                    ).as_error(),
                    ParentEnum::Record(record) => match record.id {
                        None => return creation_err(
                            String::from(
                                "Could not create account. Invalid account type.",
                            )
                        ).as_error(),
                        Some(res) => res.to_string(),
                    }
                }),
                vec![],
            )
            .include(account_model::include!({ owner account_type }))
            .exec()
            .await;

        match response {
            Ok(record) => Ok(GetOrCreateResponseKind::Created(Account {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                name: record.name,
                is_active: record.is_active,
                is_checked: record.is_checked,
                is_archived: record.is_archived,
                verbose_status: Some(VerboseStatus::from_flags(
                    record.is_active,
                    record.is_checked,
                    record.is_archived,
                )),
                owner: ParentEnum::Record(User {
                    id: Some(Uuid::parse_str(&record.owner.id).unwrap()),
                    username: record.owner.username,
                    email: Email::from_string(record.owner.email)?,
                    first_name: Some(record.owner.first_name),
                    last_name: Some(record.owner.last_name),
                    is_active: record.owner.is_active,
                    created: record.owner.created.into(),
                    updated: match record.owner.updated {
                        None => None,
                        Some(date) => Some(date.with_timezone(&Local)),
                    },
                }),
                account_type: ParentEnum::Record(AccountType {
                    id: Some(Uuid::parse_str(&record.account_type.id).unwrap()),
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
                    Some(date) => Some(date.with_timezone(&Local)),
                },
            })),
            Err(err) => {
                return creation_err(
                    format!(
                        "Unexpected error detected on update record: {}",
                        err
                    ),
                ).as_error();
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHODS
    // ? -----------------------------------------------------------------------

    async fn create(
        &self,
        _: Account,
    ) -> Result<CreateResponseKind<Account>, MappedErrors> {
        panic!("Not implemented method `create`.")
    }
}
