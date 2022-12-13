use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

/// This enumerator are used to standardize errors codes dispatched during the
/// `MappedErrors` struct usage.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorType {
    // Default option
    UndefinedError,

    // Crud errors
    CreationError,
    UpdatingError,
    FetchingError,
    DeletionError,

    // Clean architecture errors
    UseCaseError,

    // General errors
    ExecutionError,

    // Argument errors
    InvalidRepositoryError,
    InvalidArgumentError,
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ErrorType::UndefinedError => write!(f, "undefined-error"),
            ErrorType::CreationError => write!(f, "creation-error"),
            ErrorType::UpdatingError => write!(f, "updating-error"),
            ErrorType::FetchingError => write!(f, "fetching-error"),
            ErrorType::DeletionError => write!(f, "deletion-error"),
            ErrorType::UseCaseError => write!(f, "use-case-error"),
            ErrorType::ExecutionError => write!(f, "execution-error"),
            ErrorType::InvalidRepositoryError => {
                write!(f, "invalid-repository-error")
            }
            ErrorType::InvalidArgumentError => {
                write!(f, "invalid-argument-error")
            }
        }
    }
}

impl FromStr for ErrorType {
    type Err = ();

    fn from_str(s: &str) -> Result<ErrorType, ()> {
        match s {
            "undefined-error" => Ok(ErrorType::UndefinedError),
            "creation-error" => Ok(ErrorType::CreationError),
            "updating-error" => Ok(ErrorType::UpdatingError),
            "fetching-error" => Ok(ErrorType::FetchingError),
            "deletion-error" => Ok(ErrorType::DeletionError),
            "use-case-error" => Ok(ErrorType::UseCaseError),
            "execution-error" => Ok(ErrorType::ExecutionError),
            "invalid-repository-error" => Ok(ErrorType::InvalidRepositoryError),
            "invalid-argument-error" => Ok(ErrorType::InvalidArgumentError),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct MappedErrors {
    msg: String,
    error_type: ErrorType,
}

impl Display for MappedErrors {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "({}): {}", self.error_type, self.msg)
    }
}

impl MappedErrors {
    fn new(
        msg: String,
        exp: Option<bool>,
        prev: Option<MappedErrors>,
        error_type: ErrorType,
    ) -> MappedErrors {
        if !exp.unwrap_or(true) {
            error!("Unexpected error: ({}){}", &error_type, &msg);
        } else {
            warn!("{:?}", &msg);
        }

        if prev.is_some() {
            let updated_msg = format!(
                "[Current error] {:?}: [Previous error] {:?}",
                msg,
                &prev.unwrap().msg
            );

            return MappedErrors::new(updated_msg, exp, None, error_type);
        }

        MappedErrors { msg, error_type }
    }
}

/// Such functions should ve used over the raw `MappedErrors` struct to resolve
/// specific errors.

pub fn creation_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::CreationError)
}

pub fn updating_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::UpdatingError)
}

pub fn fetching_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::FetchingError)
}

pub fn deletion_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::DeletionError)
}

pub fn use_case_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::UseCaseError)
}

pub fn execution_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::ExecutionError)
}

pub fn invalid_repo_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::InvalidRepositoryError)
}

pub fn invalid_arg_err(
    msg: String,
    exp: Option<bool>,
    prev: Option<MappedErrors>,
) -> MappedErrors {
    MappedErrors::new(msg, exp, prev, ErrorType::InvalidArgumentError)
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        creation_err, deletion_err, fetching_err, updating_err, ErrorType,
    };

    #[test]
    fn creation_err_works() {
        assert_eq!(
            creation_err("msg".to_string(), None, None).error_type,
            ErrorType::CreationError
        );
    }

    #[test]
    fn deletion_err_works() {
        assert_eq!(
            deletion_err("msg".to_string(), None, None).error_type,
            ErrorType::DeletionError
        );
    }

    #[test]
    fn fetching_err_works() {
        assert_eq!(
            fetching_err("msg".to_string(), None, None).error_type,
            ErrorType::FetchingError
        );
    }

    #[test]
    fn updating_err_works() {
        assert_eq!(
            updating_err("msg".to_string(), None, None).error_type,
            ErrorType::UpdatingError
        );
    }
}
