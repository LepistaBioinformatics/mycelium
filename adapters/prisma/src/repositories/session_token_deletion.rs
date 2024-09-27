use crate::{
    prisma::session_token as session_token_model,
    repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::SessionTokenDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = SessionTokenDeletion)]
pub struct SessionTokenDeletionSqlDbRepository {}

#[async_trait]
impl SessionTokenDeletion for SessionTokenDeletionSqlDbRepository {
    async fn delete(
        &self,
        session_key: String,
    ) -> Result<DeletionResponseKind<String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return deletion_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        match client
            .session_token()
            .delete(session_token_model::key::equals(session_key.to_owned()))
            .exec()
            .await
        {
            Err(err) => Ok(DeletionResponseKind::NotDeleted(
                session_key,
                err.to_string(),
            )),
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
