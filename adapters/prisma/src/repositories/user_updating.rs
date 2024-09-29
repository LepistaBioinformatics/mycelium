use crate::{
    prisma::{
        identity_provider as identity_provider_model, user as user_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{PasswordHash, User},
    },
    entities::UserUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
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
        user: User,
    ) -> Result<UpdatingResponseKind<User>, MappedErrors> {
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

        let user_id = match user.id {
            None => {
                return updating_err(String::from(
                    "Unable to update user. Invalid record ID",
                ))
                .as_error()
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

                Ok(UpdatingResponseKind::Updated(
                    User::new(
                        Some(id.unwrap()),
                        record.username,
                        Email::from_string(record.email)?,
                        Some(record.first_name),
                        Some(record.last_name),
                        record.is_active,
                        record.created.into(),
                        match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                        None,
                        None,
                    )
                    .with_principal(record.is_principal),
                ))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        user_id
                    ))
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    async fn update_password(
        &self,
        user_id: Uuid,
        new_password: PasswordHash,
    ) -> Result<UpdatingResponseKind<bool>, MappedErrors> {
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

        match client
            .identity_provider()
            .update(
                identity_provider_model::user_id::equals(user_id.to_string()),
                vec![identity_provider_model::password_hash::set(Some(
                    new_password.hash,
                ))],
            )
            .exec()
            .await
        {
            Ok(_) => Ok(UpdatingResponseKind::Updated(true)),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid user type: {:?}",
                        user_id
                    ))
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {err}",
                ))
                .as_error();
            }
        }
    }
}
