use crate::{prisma::user as user_model, repositories::connector::get_client};

use async_trait::async_trait;
use chrono::Local;
use clean_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{email::Email, user::User},
    entities::UserRegistration,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserRegistration)]
pub struct UserRegistrationSqlDbRepository {}

#[async_trait]
impl UserRegistration for UserRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        user: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
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
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let response = client
            .user()
            .find_first(vec![user_model::email::equals(user.email.get_email())])
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id);

                return Ok(GetOrCreateResponseKind::NotCreated(
                    User {
                        id: Some(id.unwrap()),
                        username: record.username,
                        email: Email::from_string(record.email)?,
                        first_name: Some(record.first_name),
                        last_name: Some(record.last_name),
                        is_active: record.is_active,
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                    },
                    "User already exists".to_string(),
                ));
            }
            None => (),
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        let response = client
            .user()
            .create(
                user.username,
                user.email.get_email(),
                user.first_name.unwrap_or(String::from("")),
                user.last_name.unwrap_or(String::from("")),
                vec![],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id);

                Ok(GetOrCreateResponseKind::Created(User {
                    id: Some(id.unwrap()),
                    username: record.username,
                    email: Email::from_string(record.email)?,
                    first_name: Some(record.first_name),
                    last_name: Some(record.last_name),
                    is_active: record.is_active,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(date) => Some(date.with_timezone(&Local)),
                    },
                }))
            }
            Err(err) => {
                return Err(creation_err(
                    format!(
                        "Unexpected error detected on update record: {}",
                        err
                    ),
                    Some(false),
                    None,
                ));
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHODS
    // ? -----------------------------------------------------------------------

    async fn create(
        &self,
        user: User,
    ) -> Result<CreateResponseKind<User>, MappedErrors> {
        self.create(user).await
    }
}
