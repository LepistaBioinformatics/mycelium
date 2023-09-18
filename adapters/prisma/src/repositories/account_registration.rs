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
    dtos::{enums::ParentEnum, Children},
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{
        account::{Account, AccountType, VerboseStatus},
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::User,
    },
    entities::AccountRegistration,
};
use prisma_client_rust::or;
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
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let owner = match account.owners.to_owned() {
            Children::Ids(_) => {
                return creation_err(String::from(
                    "Could not create account. User e-mail invalid.",
                ))
                .as_error()
            }
            Children::Records(res) => res.first().unwrap().to_owned(),
        };

        let account_type_id = match account.account_type {
            ParentEnum::Id(id) => id.to_string(),
            ParentEnum::Record(record) => match record.id {
                Some(res) => res.to_string(),
                None => {
                    return creation_err(String::from(
                        "Could not create account. Invalid account type.",
                    ))
                    .as_error()
                }
            },
        };

        let response = client
            .account()
            .find_first(vec![or![
                account_model::name::equals(account.name.to_owned()),
                account_model::owners::some(vec![user_model::email::equals(
                    owner.email.get_email(),
                )]),
            ]])
            .include(account_model::include!({ owners account_type }))
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let id = Uuid::parse_str(&record.id).unwrap();

                return Ok(GetOrCreateResponseKind::NotCreated(
                    Account {
                        id: Some(id),
                        name: record.name,
                        is_active: record.is_active,
                        is_checked: record.is_checked,
                        is_archived: record.is_archived,
                        verbose_status: Some(VerboseStatus::from_flags(
                            record.is_active,
                            record.is_checked,
                            record.is_archived,
                        )),
                        is_default: false,
                        owners: Children::Records(
                            record
                                .owners
                                .into_iter()
                                .map(|owner| User {
                                    id: Some(
                                        Uuid::parse_str(&owner.id).unwrap(),
                                    ),
                                    username: owner.username,
                                    email: Email::from_string(owner.email)
                                        .unwrap(),
                                    first_name: Some(owner.first_name),
                                    last_name: Some(owner.last_name),
                                    provider: None,
                                    is_active: owner.is_active,
                                    created: owner.created.into(),
                                    updated: match owner.updated {
                                        None => None,
                                        Some(date) => {
                                            Some(date.with_timezone(&Local))
                                        }
                                    },
                                    account: Some(ParentEnum::Id(id)),
                                })
                                .collect::<Vec<User>>(),
                        ),
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

        match client
            ._transaction()
            .run(|client| async move {
                let account = client
                    .account()
                    .create(
                        account.name,
                        account_type_model::id::equals(account_type_id),
                        vec![
                            account_model::is_active::set(account.is_active),
                            account_model::is_checked::set(account.is_checked),
                            account_model::is_archived::set(
                                account.is_archived,
                            ),
                        ],
                    )
                    .include(account_model::include!({ owners account_type }))
                    .exec()
                    .await?;

                client
                    .user()
                    .create(
                        owner.username,
                        owner.email.get_email(),
                        owner.first_name.unwrap_or(String::from("")),
                        owner.last_name.unwrap_or(String::from("")),
                        account_model::id::equals(account.to_owned().id),
                        vec![],
                    )
                    .exec()
                    .await
                    .map(|owner| (account, owner))
            })
            .await
        {
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .with_code(NativeErrorCodes::MYC00002.as_str())
                .as_error();
            }
            Ok((account, _)) => {
                let id = Uuid::parse_str(&account.id).unwrap();

                Ok(GetOrCreateResponseKind::Created(Account {
                    id: Some(id),
                    name: account.name,
                    is_active: account.is_active,
                    is_checked: account.is_checked,
                    is_archived: account.is_archived,
                    verbose_status: Some(VerboseStatus::from_flags(
                        account.is_active,
                        account.is_checked,
                        account.is_archived,
                    )),
                    is_default: false,
                    owners: Children::Records(
                        account
                            .owners
                            .into_iter()
                            .map(|owner| User {
                                id: Some(Uuid::parse_str(&owner.id).unwrap()),
                                username: owner.username,
                                email: Email::from_string(owner.email).unwrap(),
                                first_name: Some(owner.first_name),
                                last_name: Some(owner.last_name),
                                provider: None,
                                is_active: owner.is_active,
                                created: owner.created.into(),
                                updated: match owner.updated {
                                    None => None,
                                    Some(date) => {
                                        Some(date.with_timezone(&Local))
                                    }
                                },
                                account: Some(ParentEnum::Id(id)),
                            })
                            .collect::<Vec<User>>(),
                    ),
                    account_type: ParentEnum::Record(AccountType {
                        id: Some(
                            Uuid::parse_str(&account.account_type.id).unwrap(),
                        ),
                        name: account.account_type.name,
                        description: account.account_type.description,
                        is_subscription: account.account_type.is_subscription,
                        is_manager: account.account_type.is_manager,
                        is_staff: account.account_type.is_staff,
                    }),
                    guest_users: None,
                    created: account.created.into(),
                    updated: match account.updated {
                        None => None,
                        Some(date) => Some(date.with_timezone(&Local)),
                    },
                }))
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
