use crate::{
    prisma::error_code as error_code_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::CreateResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeRegistration,
};
use prisma_client_rust::prisma_errors::query_engine::UniqueKeyViolation;
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = ErrorCodeRegistration)]
pub struct ErrorCodeRegistrationSqlDbRepository {}

#[async_trait]
impl ErrorCodeRegistration for ErrorCodeRegistrationSqlDbRepository {
    async fn create(
        &self,
        error_code: ErrorCode,
    ) -> Result<CreateResponseKind<ErrorCode>, MappedErrors> {
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

        match client
            .error_code()
            .create(
                error_code.prefix.to_owned(),
                error_code.message.to_owned(),
                vec![
                    error_code_model::details::set(
                        error_code.details.to_owned(),
                    ),
                    error_code_model::is_internal::set(
                        error_code.is_internal.to_owned(),
                    ),
                ],
            )
            .exec()
            .await
        {
            Err(err) => {
                if err.is_prisma_error::<UniqueKeyViolation>() {
                    return Ok(CreateResponseKind::NotCreated(
                        error_code,
                        "Unique key violation".to_string(),
                    ));
                };

                creation_err(format!(
                    "Error while creating error code: {}",
                    err.to_string()
                ))
                .as_error()
            }

            Ok(res) => Ok(CreateResponseKind::Created(ErrorCode {
                prefix: res.prefix,
                code: res.code,
                message: res.message,
                details: res.details,
                is_internal: res.is_internal,
            })),
        }
    }
}
