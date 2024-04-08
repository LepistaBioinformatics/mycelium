use crate::{
    prisma::{
        identity_provider::{self as identity_provider_model},
        user as user_model, QueryMode,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{PasswordHash, Provider, User},
    },
    entities::UserFetching,
};
use mycelium_base::{
    dtos::Parent,
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::and;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserFetching)]
pub struct UserFetchingSqlDbRepository {}

#[async_trait]
impl UserFetching for UserFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Option<Uuid>,
        email: Option<Email>,
        password_hash: Option<String>,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build query params
        // ? -------------------------------------------------------------------

        let mut query_stmt = vec![];

        if id.is_some() {
            query_stmt.push(user_model::id::equals(id.unwrap().to_string()))
        }

        if let Some(email) = email {
            query_stmt.push(and![
                user_model::email::mode(QueryMode::Insensitive),
                user_model::email::equals(email.get_email())
            ])
        }

        if password_hash.is_some() {
            query_stmt.push(user_model::provider::is(vec![
                identity_provider_model::password_hash::equals(
                    password_hash.to_owned(),
                ),
            ]))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .user()
            .find_first(query_stmt)
            .include(user_model::include!({ provider }))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(None)),
                Some(record) => {
                    if record.provider.is_none() {
                        return fetching_err(String::from(
                            "Unexpected error on parse user: {:?}",
                        ))
                        .as_error();
                    }

                    let record_provider = &record.provider.unwrap();
                    let record_password_hash = &record_provider.password_hash;
                    let record_password_salt = &record_provider.password_salt;
                    let record_provider_name = &record_provider.name;

                    let provider = {
                        if record_password_hash.is_some() &&
                            record_password_salt.is_some()
                        {
                            Provider::Internal(PasswordHash {
                                hash: record_password_hash.clone().unwrap(),
                                salt: record_password_salt.clone().unwrap(),
                            })
                        } else if record_provider_name.is_some() {
                            Provider::External(
                                record_provider_name.clone().unwrap(),
                            )
                        } else {
                            return fetching_err(String::from(
                                "Unexpected error on parse user email: {:?}",
                            ))
                            .as_error();
                        }
                    };

                    Ok(FetchResponseKind::Found(
                        User::new(
                            Some(Uuid::parse_str(&record.id).unwrap()),
                            record.username,
                            Email::from_string(record.email).unwrap(),
                            Some(record.first_name),
                            Some(record.last_name),
                            record.is_active,
                            record.created.into(),
                            match record.updated {
                                None => None,
                                Some(date) => Some(date.with_timezone(&Local)),
                            },
                            match &record.account_id {
                                Some(id) => Some(Parent::Id(
                                    Uuid::parse_str(id).unwrap(),
                                )),
                                None => None,
                            },
                            Some(provider),
                        )
                        .with_principal(record.is_principal),
                    ))
                }
            },
        }
    }
}
