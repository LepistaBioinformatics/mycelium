use crate::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeRegistration,
};

use futures::future::join_all;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use tracing::error;

/// Statistics about the error persistence process
///
/// This structure will contain the number of errors that were persisted and
/// the list of errors that were not persisted.
pub struct ErrorPersistenceStatistics {
    pub persisted_errors: i32,
    pub unpersisted_errors: Vec<ErrorCode>,
}

/// Persist all native error codes in the data repository
#[tracing::instrument(name = "batch_register_native_error_codes", skip_all)]
pub async fn batch_register_native_error_codes(
    error_code_registration_repo: Box<&dyn ErrorCodeRegistration>,
) -> Result<ErrorPersistenceStatistics, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to register errors
    // ? -----------------------------------------------------------------------

    let mut persisted_errors: i32 = 0;
    let mut unpersisted_errors = Vec::<ErrorCode>::new();

    let persisting_operations = NativeErrorCodes::to_error_codes()?
        .into_iter()
        .map(|entry| {
            let (_, error) = entry;

            error_code_registration_repo.create(error.to_owned())
        });

    join_all(persisting_operations)
        .await
        .into_iter()
        .for_each(|response| {
            if let Err(err) = response {
                return error!("{}", err.to_string());
            }

            match response.unwrap() {
                CreateResponseKind::NotCreated(error, msg) => {
                    unpersisted_errors.push(error);
                    error!("Error detected on try to register error: {msg}")
                }
                CreateResponseKind::Created(_) => {
                    persisted_errors += 1;
                }
            }
        });

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ErrorPersistenceStatistics {
        persisted_errors,
        unpersisted_errors,
    })
}
