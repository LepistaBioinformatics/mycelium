use crate::{
    prisma::session_token as session_token_model,
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes,
    entities::SessionTokenRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = SessionTokenRegistration)]
pub struct SessionTokenRegistrationSqlDbRepository {}

#[async_trait]
impl SessionTokenRegistration for SessionTokenRegistrationSqlDbRepository {
    async fn create(
        &self,
        session_key: String,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<bool>, MappedErrors> {
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

        let response = client
            .session_token()
            .create(
                session_key,
                vec![session_token_model::expiration::set(DateTime::from(
                    expires,
                ))],
            )
            .exec()
            .await;

        match response {
            Ok(_) => Ok(CreateResponseKind::Created(true)),
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {err}"
                ))
                .as_error();
            }
        }
    }
}
