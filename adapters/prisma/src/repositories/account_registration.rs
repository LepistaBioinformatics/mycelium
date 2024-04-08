use crate::{
    prisma::{
        account as account_model, account_type as account_type_model,
        user as user_model, QueryMode,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{
        account::{Account, AccountType, VerboseStatus},
        email::Email,
        native_error_codes::NativeErrorCodes,
        tag::Tag,
        user::User,
    },
    entities::AccountRegistration,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use prisma_client_rust::{and, or};
use serde_json::from_value;
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
        user_exists: bool,
        omit_user_creation: bool,
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

        let account_type_id = match account.account_type {
            Parent::Id(id) => id.to_string(),
            Parent::Record(record) => match record.id {
                Some(res) => res.to_string(),
                None => {
                    return creation_err(String::from(
                        "Could not create account. Invalid account type.",
                    ))
                    .as_error()
                }
            },
        };

        let emails = match account.owners.to_owned() {
            Children::Ids(_) => vec![],
            Children::Records(res) => res
                .into_iter()
                .map(|user| user.email.get_email())
                .collect::<Vec<String>>(),
        };

        let response = client
            .account()
            .find_first(vec![or![
                account_model::slug::equals(account.name.to_owned()),
                account_model::owners::some(vec![and![
                    user_model::email::mode(QueryMode::Insensitive),
                    user_model::email::in_vec(emails),
                ]]),
            ]])
            .include(account_model::include!({
                owners
                account_type
                tags: select {
                    id
                    value
                    meta
                }
            }))
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let id = Uuid::parse_str(&record.id).unwrap();

                return Ok(GetOrCreateResponseKind::NotCreated(
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
                        account_type: Parent::Record(AccountType {
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

        if omit_user_creation {
            //
            // User creation is omitted, so we just create the account
            //
            match client
                ._transaction()
                .run(|client| async move {
                    client
                        .account()
                        .create(
                            account.name,
                            account.slug,
                            account_type_model::id::equals(account_type_id),
                            vec![
                                account_model::is_active::set(
                                    account.is_active,
                                ),
                                account_model::is_checked::set(
                                    account.is_checked,
                                ),
                                account_model::is_archived::set(
                                    account.is_archived,
                                ),
                                account_model::is_default::set(
                                    account.is_default,
                                ),
                            ],
                        )
                        .include(account_model::include!({
                            owners
                            account_type
                            tags: select {
                                id
                                value
                                meta
                            }
                        }))
                        .exec()
                        .await
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
                Ok(account) => {
                    let id = Uuid::parse_str(&account.id).unwrap();

                    return Ok(GetOrCreateResponseKind::Created(Account {
                        id: Some(id),
                        name: account.name,
                        slug: account.slug,
                        tags: match account.tags.len() {
                            0 => None,
                            _ => Some(
                                account
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
                        is_active: account.is_active,
                        is_checked: account.is_checked,
                        is_archived: account.is_archived,
                        verbose_status: Some(VerboseStatus::from_flags(
                            account.is_active,
                            account.is_checked,
                            account.is_archived,
                        )),
                        is_default: account.is_default,
                        owners: Children::Records(
                            account
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
                        account_type: Parent::Record(AccountType {
                            id: Some(
                                Uuid::parse_str(&account.account_type.id)
                                    .unwrap(),
                            ),
                            name: account.account_type.name,
                            description: account.account_type.description,
                            is_subscription: account
                                .account_type
                                .is_subscription,
                            is_manager: account.account_type.is_manager,
                            is_staff: account.account_type.is_staff,
                        }),
                        guest_users: None,
                        created: account.created.into(),
                        updated: match account.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                    }));
                }
            }
        } else {
            //
            // User creation is not omitted, so we create the account and the
            // user.
            //
            let owner = match account.owners.to_owned() {
                Children::Ids(_) => {
                    return creation_err(String::from(
                        "Could not create account. User e-mail invalid.",
                    ))
                    .as_error()
                }
                Children::Records(res) => res.first().unwrap().to_owned(),
            };
            //
            // User creation is not omitted, so we create the account and the
            // user.
            //
            match client
                ._transaction()
                .run(|client| async move {
                    let account = client
                        .account()
                        .create(
                            account.name,
                            account.slug,
                            account_type_model::id::equals(account_type_id),
                            vec![
                                account_model::is_active::set(
                                    account.is_active,
                                ),
                                account_model::is_checked::set(
                                    account.is_checked,
                                ),
                                account_model::is_archived::set(
                                    account.is_archived,
                                ),
                                account_model::is_default::set(
                                    account.is_default,
                                ),
                            ],
                        )
                        .include(account_model::include!({
                            owners
                            account_type
                            tags: select {
                                id
                                value
                                meta
                            }
                        }))
                        .exec()
                        .await?;

                    if user_exists && owner.id.is_some() {
                        client
                            .user()
                            .update(
                                user_model::id::equals(
                                    owner.id.unwrap().to_string(),
                                ),
                                vec![
                                    user_model::account_id::set(Some(
                                        account.to_owned().id.to_string(),
                                    )),
                                    user_model::is_active::set(owner.is_active),
                                ],
                            )
                            .exec()
                            .await
                            .map(|owner| (owner, account))
                    } else {
                        client
                            .user()
                            .create(
                                owner.to_owned().username,
                                owner.to_owned().email.get_email(),
                                owner
                                    .to_owned()
                                    .first_name
                                    .unwrap_or(String::from("")),
                                owner
                                    .to_owned()
                                    .last_name
                                    .unwrap_or(String::from("")),
                                vec![
                                    user_model::account_id::set(Some(
                                        account.to_owned().id.to_string(),
                                    )),
                                    user_model::is_principal::set(
                                        owner.is_principal(),
                                    ),
                                ],
                            )
                            .exec()
                            .await
                            .map(|owner| (owner, account))
                    }
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
                Ok((owner, account)) => {
                    let id = Uuid::parse_str(&account.id).unwrap();

                    Ok(GetOrCreateResponseKind::Created(Account {
                        id: Some(id),
                        name: account.name,
                        slug: account.slug,
                        tags: match account.tags.len() {
                            0 => None,
                            _ => Some(
                                account
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
                        is_active: account.is_active,
                        is_checked: account.is_checked,
                        is_archived: account.is_archived,
                        verbose_status: Some(VerboseStatus::from_flags(
                            account.is_active,
                            account.is_checked,
                            account.is_archived,
                        )),
                        is_default: account.is_default,
                        owners: Children::Records(vec![User::new(
                            Some(Uuid::parse_str(&owner.id).unwrap()),
                            owner.username,
                            Email::from_string(owner.email).unwrap(),
                            Some(owner.first_name),
                            Some(owner.last_name),
                            owner.is_active,
                            owner.created.into(),
                            match owner.updated {
                                None => None,
                                Some(date) => Some(date.with_timezone(&Local)),
                            },
                            Some(Parent::Id(id)),
                            None,
                        )
                        .with_principal(owner.is_principal)]),
                        account_type: Parent::Record(AccountType {
                            id: Some(
                                Uuid::parse_str(&account.account_type.id)
                                    .unwrap(),
                            ),
                            name: account.account_type.name,
                            description: account.account_type.description,
                            is_subscription: account
                                .account_type
                                .is_subscription,
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
