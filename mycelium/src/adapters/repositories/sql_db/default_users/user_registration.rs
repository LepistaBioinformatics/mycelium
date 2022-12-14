use crate::{
    adapters::repositories::sql_db::connector::get_client,
    domain::{
        dtos::{email::EmailDTO, user::UserDTO},
        entities::default_users::user_registration::UserRegistration,
    },
};

use agrobase::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use async_trait::async_trait;
use chrono::Local;
use myc_prisma::prisma::user as user_model;
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
        user: UserDTO,
    ) -> Result<GetOrCreateResponseKind<UserDTO>, MappedErrors> {
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
                    UserDTO {
                        id: Some(id.unwrap()),
                        username: record.username,
                        email: EmailDTO::from_string(record.email).unwrap(),
                        first_name: Some(record.first_name),
                        last_name: Some(record.last_name),
                        is_active: record.is_active,
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                    },
                    "Customer already exists".to_string(),
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
                user.first_name.unwrap(),
                user.last_name.unwrap(),
                vec![],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id);

                Ok(GetOrCreateResponseKind::Created(UserDTO {
                    id: Some(id.unwrap()),
                    username: record.username,
                    email: EmailDTO::from_string(record.email).unwrap(),
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
        user: UserDTO,
    ) -> Result<CreateResponseKind<UserDTO>, MappedErrors> {
        self.create(user).await
    }
}
