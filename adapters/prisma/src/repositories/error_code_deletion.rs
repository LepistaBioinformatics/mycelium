use crate::{
    prisma::error_code as error_code_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::deletion_err, MappedErrors},
};
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::ErrorCodeDeletion,
};
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = ErrorCodeDeletion)]
pub struct ErrorCodeDeletionDeletionSqlDbRepository {}

#[async_trait]
impl ErrorCodeDeletion for ErrorCodeDeletionDeletionSqlDbRepository {
    async fn delete(
        &self,
        prefix: String,
        code: i32,
    ) -> Result<DeletionResponseKind<(String, i32)>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return deletion_err(String::from(
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

        match client
            .error_code()
            .delete(error_code_model::prefix_code(
                prefix.to_owned(),
                code.to_owned(),
            ))
            .exec()
            .await
        {
            Err(err) => Ok(DeletionResponseKind::NotDeleted(
                (prefix, code),
                err.to_string(),
            )),
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
