use crate::{
    adapters::repositories::sql_db::connector::get_client,
    domain::{
        dtos::{email::EmailDTO, user::UserDTO},
        entities::default_users::user_updating::UserUpdating,
    },
};

use agrobase::{
    entities::default_response::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use async_trait::async_trait;
use chrono::Local;
use myc_prisma::prisma::user as user_model;
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserUpdating)]
pub struct UserUpdatingSqlDbRepository {}

#[async_trait]
impl UserUpdating for UserUpdatingSqlDbRepository {
    async fn update(
        &self,
        user: UserDTO,
    ) -> Result<UpdatingResponseKind<UserDTO>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(updating_err(
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
        // ? Try to update record
        // ? -------------------------------------------------------------------

        let user_id = match user.id {
            None => {
                return Err(updating_err(
                    String::from("Unable to update user. Invalid record ID"),
                    None,
                    None,
                ))
            }
            Some(res) => res,
        };

        let response = client
            .user()
            .update(
                user_model::id::equals(user_id.to_string()),
                vec![
                    user_model::username::set(user.username),
                    user_model::first_name::set(user.first_name.unwrap()),
                    user_model::last_name::set(user.last_name.unwrap()),
                    user_model::is_active::set(user.is_active),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id);

                Ok(UpdatingResponseKind::Updated(UserDTO {
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
                if err.is_prisma_error::<RecordNotFound>() {
                    return Err(updating_err(
                        format!("Invalid primary key: {:?}", user_id),
                        None,
                        None,
                    ));
                };

                return Err(updating_err(
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
}
