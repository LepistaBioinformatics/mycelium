use crate::{
    prisma::error_code as error_code_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::UpdatingResponseKind,
    utils::errors::{factories::updating_err, MappedErrors},
};
use myc_core::domain::{
    dtos::error_code::ErrorCode, entities::ErrorCodeUpdating,
};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = ErrorCodeUpdating)]
pub struct ErrorCodeUpdatingSqlDbRepository {}

#[async_trait]
impl ErrorCodeUpdating for ErrorCodeUpdatingSqlDbRepository {
    async fn update(
        &self,
        error_code: ErrorCode,
    ) -> Result<UpdatingResponseKind<ErrorCode>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code("MYC00001".to_string())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------

        match client
            .error_code()
            .update(
                error_code_model::prefix_code(
                    error_code.prefix.to_owned(),
                    error_code.code.to_owned(),
                ),
                vec![
                    error_code_model::message::set(
                        error_code.message.to_owned(),
                    ),
                    error_code_model::details::set(
                        error_code.details.to_owned(),
                    ),
                    error_code_model::is_internal::set(error_code.is_internal),
                ],
            )
            .exec()
            .await
        {
            Ok(record) => Ok(UpdatingResponseKind::Updated(ErrorCode {
                prefix: record.prefix,
                code: record.code,
                message: record.message,
                details: record.details,
                is_internal: record.is_internal,
            })),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary keys combination: {}, {}",
                        error_code.prefix, error_code.code,
                    ))
                    .with_exp_false()
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
