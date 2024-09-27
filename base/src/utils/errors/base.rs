use log::{debug, error, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

/// This enumerator are used to standardize errors codes dispatched during the
/// `MappedErrors` struct usage.
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorType {
    /// This error type is used when the error type is not defined. This is the
    /// default value for the `ErrorType` enum.
    ///
    /// Related: Undefined
    UndefinedError,

    /// This error type is used when a creation error occurs.
    ///
    /// Related: CRUD
    CreationError,

    /// This error type is used when an updating error occurs.
    ///
    /// Related: CRUD
    UpdatingError,

    /// This error type is used when an updating many error occurs.
    ///
    /// Related: CRUD
    UpdatingManyError,

    /// This error type is used when a fetching error occurs.
    ///
    /// Related: CRUD
    FetchingError,

    /// This error type is used when a deletion error occurs.
    ///
    /// Related: CRUD
    DeletionError,

    /// This error type is used when a use case error occurs.
    ///
    /// Related: Use Case
    UseCaseError,

    /// This error type is used when an execution error occurs. This error type
    /// is used when the error is not related to a specific action.
    ///
    /// Related: Execution
    ExecutionError,

    /// This error type is used when an invalid data repository error occurs.
    ///
    /// Related: Data Repository
    InvalidRepositoryError,

    /// This error type is used when an invalid argument error occurs.
    ///
    /// Related: Argument
    InvalidArgumentError,

    /// This error type is used when an error occurs in the data transfer layer.
    ///
    /// Related: Data Transfer Objects
    DataTransferLayerError,

    /// This error type is used when a general error occurs.
    ///
    /// Related: General
    GeneralError(String),
}

impl ErrorType {
    fn default() -> Self {
        Self::UndefinedError
    }
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ErrorType::UndefinedError => write!(f, "undefined-error"),
            ErrorType::CreationError => write!(f, "creation-error"),
            ErrorType::UpdatingError => write!(f, "updating-error"),
            ErrorType::UpdatingManyError => write!(f, "updating-many-error"),
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
            ErrorType::DataTransferLayerError => {
                write!(f, "data-transfer-layer-error")
            }
            ErrorType::GeneralError(msg) => write!(f, "{}", msg),
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
            "updating-many-error" => Ok(ErrorType::UpdatingManyError),
            "fetching-error" => Ok(ErrorType::FetchingError),
            "deletion-error" => Ok(ErrorType::DeletionError),
            "use-case-error" => Ok(ErrorType::UseCaseError),
            "execution-error" => Ok(ErrorType::ExecutionError),
            "invalid-repository-error" => Ok(ErrorType::InvalidRepositoryError),
            "invalid-argument-error" => Ok(ErrorType::InvalidArgumentError),
            "data-transfer-layer-error" => {
                Ok(ErrorType::DataTransferLayerError)
            }
            other => Ok(ErrorType::GeneralError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCodes {
    Codes(Vec<String>),
    Unmapped,
}

impl ErrorCodes {
    pub fn default() -> ErrorCodes {
        ErrorCodes::Unmapped
    }
}

impl Display for ErrorCodes {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ErrorCodes::Codes(codes) => {
                write!(f, "{}", codes.join(MappedErrors::codes_delimiter()))
            }
            ErrorCodes::Unmapped => write!(f, "unmapped"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MappedErrors {
    /// This field contains the error message.
    msg: String,

    /// This field contains the error type. This field is used to standardize
    /// errors codes.
    error_type: ErrorType,

    /// If dispatched error is expected or not.
    expected: bool,

    /// This field contains the error code. This field is used to standardize
    /// errors evaluation in downstream applications.
    codes: ErrorCodes,
}

impl Error for MappedErrors {}

impl Display for MappedErrors {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let code_key = MappedErrors::code_key();
        let error_type_key = MappedErrors::error_type_key();

        let code_value = match self.codes.to_owned() {
            ErrorCodes::Codes(codes) => codes.join(Self::codes_delimiter()),
            ErrorCodes::Unmapped => String::from("none"),
        };

        write!(
            f,
            "[{}={}{}{}={}] {}",
            code_key,
            code_value,
            Self::msg_paras_delimiter(),
            error_type_key,
            self.error_type,
            self.msg
        )
    }
}

impl MappedErrors {
    // ? -----------------------------------------------------------------------
    // ? INSTANCE METHODS
    //
    // Getters
    //
    // ? -----------------------------------------------------------------------

    /// This method returns the error type of the current error.
    pub fn error_type(&self) -> ErrorType {
        self.error_type.to_owned()
    }

    /// This method returns the error message of the current error.
    pub fn msg(&self) -> String {
        self.msg.to_owned()
    }

    /// This method returns the error code key of the current error.
    pub fn code(&self) -> ErrorCodes {
        self.codes.to_owned()
    }

    /// This method returns the error code key of the current error.
    pub fn expected(&self) -> bool {
        self.expected.to_owned()
    }

    /// This method returns a boolean indicating if the current error is
    /// expected or not.
    pub fn has_str_code(&self, code: &str) -> bool {
        if code == "none" {
            return false;
        }

        if let ErrorCodes::Codes(inner_code) = &self.codes {
            return inner_code.into_iter().any(|i| i.as_str() == code);
        };

        return false;
    }

    pub fn is_in<T: ToString>(&self, codes: Vec<T>) -> bool {
        for code in codes {
            if self.has_str_code(code.to_string().as_str()) {
                return true;
            }
        }

        return false;
    }

    // ? -----------------------------------------------------------------------
    // ? INSTANCE METHODS
    //
    // Modifiers
    //
    // ? -----------------------------------------------------------------------

    /// Evoked when a Err return is desired.
    pub fn as_error<T>(self) -> Result<T, Self> {
        if let true = self.expected {
            debug!("{:?}", &self.to_string());
        } else {
            error!("{:?}", &self.to_string());
        }

        Err(self)
    }

    /// Dispatches an log error indicating unexpected error.
    pub fn with_exp_true(mut self) -> Self {
        self.expected = true;
        self
    }

    /// Set the error code of the current error.
    pub fn with_code<T: ToString>(mut self, code: T) -> Self {
        let binding = code.to_string();
        let code = binding.as_str();
        if code == "none" {
            return self;
        }

        let mut codes = match self.to_owned().codes {
            ErrorCodes::Codes(codes) => codes,
            ErrorCodes::Unmapped => vec![],
        };

        codes.push(code.to_string());
        codes.sort();
        codes.dedup();

        self.codes = ErrorCodes::Codes(codes);
        self
    }

    /// Include previous mapped error in message
    pub fn with_previous(mut self, prev: MappedErrors) -> Self {
        self.msg = format!(
            "[CURRENT_ERROR] {}; [PRECEDING_ERROR] {}",
            self.msg,
            &prev.to_string()
        );

        self
    }

    /// Set the error type of the current error.
    pub fn with_error_type(mut self, error_type: ErrorType) -> Self {
        self.error_type = error_type;
        self
    }

    // ? -----------------------------------------------------------------------
    // ? STRUCTURAL METHODS
    // ? -----------------------------------------------------------------------

    /// Build a anemic MappedError instance.
    pub(super) fn default(msg: String) -> Self {
        Self {
            msg: Self::sanitize_msg(msg),
            error_type: ErrorType::default(),
            expected: false,
            codes: ErrorCodes::default(),
        }
    }

    /// This method returns a new `MappedErrors` struct.
    pub(super) fn new(
        msg: String,
        exp: Option<bool>,
        prev: Option<MappedErrors>,
        error_type: ErrorType,
    ) -> Self {
        let exp = exp.unwrap_or(true);

        if !exp {
            error!("Unexpected error: ({}){}", &error_type, &msg);
        } else {
            warn!("{:?}", &msg);
        }

        if prev.is_some() {
            let updated_msg = format!(
                "[CURRENT_ERROR] {:?}; [PRECEDING_ERROR] {:?}",
                msg,
                &prev.unwrap().msg
            );

            return Self::new(updated_msg, Some(exp), None, error_type);
        }

        Self {
            msg,
            error_type,
            expected: exp,
            codes: ErrorCodes::default(),
        }
    }

    /// Set the error type of the current error.
    fn code_key() -> &'static str {
        "codes"
    }

    /// Set delimiter of the error codes.
    pub(self) fn codes_delimiter() -> &'static str {
        ","
    }

    /// Set delimiter of mapped errors string parameters.
    pub(self) fn msg_paras_delimiter() -> &'static str {
        " "
    }

    /// Set the error type of the current error.
    fn error_type_key() -> &'static str {
        "error_type"
    }

    /// Remove invalid characters from message.
    fn sanitize_msg(msg: String) -> String {
        msg.as_str().replace(";", ",").to_string()
    }

    /// This method returns a new `MappedErrors` struct from a string.
    pub fn from_str_msg(msg: String) -> Self {
        let pattern = Regex::new(
            r"^\[codes=([a-zA-Z0-9,]+)\serror_type=([a-zA-Z-]+)\]\s(.+)$",
        )
        .unwrap();

        if pattern.is_match(&msg) {
            let capture = pattern.captures(&msg).unwrap();
            let code = &capture[1];
            let msg = capture[3].to_string();

            let error_type = match ErrorType::from_str(&capture[2]) {
                Ok(error_type) => error_type,
                Err(_) => ErrorType::UndefinedError,
            };

            return MappedErrors::new(msg, None, None, error_type)
                .with_code(code);
        };

        MappedErrors::new(msg, None, None, ErrorType::UndefinedError)
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    #[test]
    fn test_error_type() {
        fn error_dispatcher() -> Result<(), super::MappedErrors> {
            Err(super::MappedErrors::new(
                "This is a test error".to_string(),
                Some(true),
                None,
                super::ErrorType::UndefinedError,
            ))
        }

        fn error_handler() -> Result<(), super::MappedErrors> {
            error_dispatcher()?;
            Ok(())
        }

        let response = error_handler().unwrap_err();

        assert_eq!(response.error_type(), super::ErrorType::UndefinedError);
    }

    #[test]
    fn test_error_msg() {
        fn error_dispatcher() -> Result<(), super::MappedErrors> {
            Err(super::MappedErrors::new(
                "This is a test error".to_string(),
                Some(true),
                None,
                super::ErrorType::UndefinedError,
            ))
        }

        fn error_handler() -> Result<(), super::MappedErrors> {
            error_dispatcher()?;
            Ok(())
        }

        let response = error_handler().unwrap_err();

        assert_eq!(
            response.to_string(),
            format!(
                "[{}=none{}{}=undefined-error] This is a test error",
                super::MappedErrors::code_key(),
                super::MappedErrors::msg_paras_delimiter(),
                super::MappedErrors::error_type_key()
            )
        );
    }

    #[test]
    fn test_from_msg() {
        let msg = format!(
            "[{}=none{}{}=undefined-error] This is a test error",
            super::MappedErrors::code_key(),
            super::MappedErrors::msg_paras_delimiter(),
            super::MappedErrors::error_type_key()
        );

        let response = super::MappedErrors::from_str_msg(msg.to_string());
        let previous = response.to_owned();

        assert_eq!(response.to_string(), msg);

        let with_previous = response.with_previous(previous);

        let from_str_msg =
            super::MappedErrors::from_str_msg(with_previous.msg());

        assert_eq!(with_previous.msg(), from_str_msg.msg());
    }

    #[test]
    fn test_has_str_code() {
        fn error_dispatcher() -> Result<(), super::MappedErrors> {
            Err(super::MappedErrors::new(
                "This is a test error".to_string(),
                Some(true),
                None,
                super::ErrorType::UndefinedError,
            ))
        }

        fn error_handler() -> Result<(), super::MappedErrors> {
            error_dispatcher()?;
            Ok(())
        }

        let response = error_handler().unwrap_err();

        assert!(!response.has_str_code("none"));
    }

    #[test]
    fn test_is_in() {
        fn error_dispatcher(
            codes: Option<Vec<String>>,
        ) -> Result<(), super::MappedErrors> {
            if codes.is_some() {
                let mut errors = super::MappedErrors::new(
                    "This is a test error".to_string(),
                    Some(true),
                    None,
                    super::ErrorType::UndefinedError,
                );

                for code in codes.unwrap() {
                    errors = errors.with_code(code.as_str());
                }

                return Err(errors);
            }

            Err(super::MappedErrors::new(
                "This is a test error".to_string(),
                Some(true),
                None,
                super::ErrorType::UndefinedError,
            ))
        }

        fn error_handler(
            codes: Option<Vec<String>>,
        ) -> Result<(), super::MappedErrors> {
            error_dispatcher(codes)?;
            Ok(())
        }

        let none_response = error_handler(None).unwrap_err();
        let some_response = error_handler(Some(vec![
            "ID00001".to_string(),
            "ID00005".to_string(),
        ]))
        .unwrap_err();

        assert!(!none_response.is_in(vec!["none", "ID00001"]));
        assert!(!some_response.is_in(vec!["none", "ID00002"]));
        assert!(!some_response.is_in(vec!["ID00002", "ID00003"]));
        assert!(some_response.is_in(vec!["none", "ID00001"]));
    }
}
