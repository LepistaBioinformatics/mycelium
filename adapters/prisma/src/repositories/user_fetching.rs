use crate::{
    prisma::{
        internal_provider as internal_provider_model, user as user_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use clean_base::{
    dtos::Parent,
    entities::FetchResponseKind,
    utils::errors::{factories::fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{email::Email, native_error_codes::NativeErrorCodes, user::User},
    entities::UserFetching,
};
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

        if email.is_some() {
            query_stmt.push(user_model::email::equals(
                email.unwrap().to_owned().get_email(),
            ))
        }

        if password_hash.is_some() {
            query_stmt.push(user_model::internal_provider::is(vec![
                internal_provider_model::password_hash::equals(
                    password_hash.unwrap().to_owned(),
                ),
            ]))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client.user().find_first(query_stmt).exec().await {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(None)),
                Some(record) => Ok(FetchResponseKind::Found(
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
                            Some(id) => {
                                Some(Parent::Id(Uuid::parse_str(id).unwrap()))
                            }
                            None => None,
                        },
                    )
                    .with_principal(record.is_principal),
                )),
            },
        }
    }
}
