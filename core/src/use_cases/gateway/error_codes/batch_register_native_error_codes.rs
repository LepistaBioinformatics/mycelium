use crate::domain::{
    dtos::{
        error_code::ErrorCode, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::ErrorCodeRegistration,
};

use clean_base::{
    entities::CreateResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use futures::future::join_all;
use log::error;

/// Statistics about the error persistence process
///
/// This structure will contain the number of errors that were persisted and
/// the list of errors that were not persisted.
pub struct ErrorPersistenceStatistics {
    pub persisted_errors: i32,
    pub unpersisted_errors: Vec<ErrorCode>,
}

/// Persist all native error codes in the data repository
pub async fn batch_register_native_error_codes(
    profile: Profile,
    error_code_registration_repo: Box<&dyn ErrorCodeRegistration>,
) -> Result<ErrorPersistenceStatistics, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register error"
                .to_string(),
        )
        .as_error();
    }

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
                    return error!(
                        "Error detected on try to register error: {msg}"
                    );
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
