use crate::{
    prisma::error_code as error_code_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
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
                .with_code(NativeErrorCodes::MYC00001)
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
                    error_code.error_number.to_owned(),
                ),
                vec![
                    error_code_model::message::set(
                        error_code.message.to_owned(),
                    ),
                    error_code_model::details::set(
                        error_code.details.to_owned(),
                    ),
                    error_code_model::is_internal::set(error_code.is_internal),
                    error_code_model::is_native::set(error_code.is_native),
                ],
            )
            .exec()
            .await
        {
            Ok(record) => {
                let mut error_code = ErrorCode {
                    prefix: record.prefix,
                    error_number: record.code,
                    code: None,
                    message: record.message,
                    details: record.details,
                    is_internal: record.is_internal,
                    is_native: record.is_native,
                };

                error_code = error_code.with_code();

                Ok(UpdatingResponseKind::Updated(error_code))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary keys combination: {}, {}",
                        error_code.prefix, error_code.error_number,
                    ))
                    .with_exp_true()
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
