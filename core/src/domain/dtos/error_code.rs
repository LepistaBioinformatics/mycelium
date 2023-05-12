use clean_base::utils::errors::{factories::execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// ErrorCode is a struct that represents an error code.
///
/// It is used to represent errors that occur in the system. Error should be
/// internal or external. Internal errors are errors that are not expected to
/// occur in the system. External errors are errors that are not expected to
/// occur in the system.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorCode {
    /// The prefix of the error.
    pub prefix: String,

    /// The code of the error.
    pub error_number: i32,

    /// A compiled string of the prefix and code.
    pub code: Option<String>,

    /// The message of the error.
    pub message: String,

    /// The details of the error.
    pub details: Option<String>,

    /// Whether the error is internal or external.
    pub is_internal: bool,

    /// Whether the error is native of mycelium or not.
    pub is_native: bool,
}

impl ErrorCode {
    // ? -----------------------------------------------------------------------
    // ? PUBLIC STRUCTURAL METHODS (CONSTRUCTORS)
    // ? -----------------------------------------------------------------------

    /// Creates a new ErrorCode with the given code and message.
    pub fn new(
        prefix: String,
        error_number: i32,
        message: String,
        is_internal: bool,
        is_native: bool,
    ) -> Result<Self, MappedErrors> {
        ErrorCode::validate_prefix(&prefix)?;

        Ok(Self {
            prefix,
            error_number,
            code: None,
            message,
            details: None,
            is_internal,
            is_native,
        })
    }

    /// Creates a new internal ErrorCode with the given code and message.
    pub fn new_internal_error(
        prefix: String,
        error_number: i32,
        message: String,
        is_native: bool,
    ) -> Result<Self, MappedErrors> {
        ErrorCode::validate_prefix(&prefix)?;

        Ok(Self {
            prefix,
            error_number,
            code: None,
            message,
            details: None,
            is_internal: true,
            is_native,
        })
    }

    /// Creates a new external ErrorCode with the given code and message.
    pub fn new_external_error(
        prefix: String,
        code: i32,
        message: String,
        is_native: bool,
    ) -> Result<Self, MappedErrors> {
        ErrorCode::validate_prefix(&prefix)?;

        Ok(Self {
            prefix,
            error_number: code,
            code: None,
            message,
            details: None,
            is_internal: false,
            is_native,
        })
    }

    // ? -----------------------------------------------------------------------
    // ? PUBLIC INSTANCE METHODS
    // ? -----------------------------------------------------------------------

    /// Creates a new ErrorCode with the given code, message, and details.
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_code(mut self) -> Self {
        self.code = Some(format!("{}{:05}", self.prefix, self.error_number));
        self
    }

    // ? -----------------------------------------------------------------------
    // ? PRIVATE STATIC METHODS
    // ? -----------------------------------------------------------------------

    /// Validates the given prefix.
    fn validate_prefix(prefix: &str) -> Result<(), MappedErrors> {
        let (min_prefix_size, max_prefix_size) =
            Self::get_min_and_max_prefix_sizes();

        if prefix.len() < min_prefix_size {
            return execution_err(format!(
                "Prefix must be longest than {min_prefix_size}."
            ))
            .as_error();
        }

        if prefix.len() > max_prefix_size {
            return execution_err(format!(
                "Prefix must be shortest than {max_prefix_size}."
            ))
            .as_error();
        }

        if !prefix
            .chars()
            .all(|c| c.is_ascii_alphabetic() && c.is_uppercase())
        {
            return execution_err("Prefix must be all alphabetic.".to_string())
                .as_error();
        }

        Ok(())
    }

    /// Gets the minimum and maximum prefix sizes.
    pub(self) fn get_min_and_max_prefix_sizes() -> (usize, usize) {
        (2, 4)
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_internal_error() {
        let error_code = ErrorCode::new_internal_error(
            "TEST".to_string(),
            1,
            "Test error.".to_string(),
            false,
        )
        .unwrap();

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.error_number, 1);
        assert_eq!(error_code.message, "Test error.");
        assert_eq!(error_code.details, None);
        assert_eq!(error_code.is_internal, true);
    }

    #[test]
    fn test_new_external_error() {
        let error_code = ErrorCode::new_external_error(
            "TEST".to_string(),
            1,
            "Test error.".to_string(),
            false,
        )
        .unwrap();

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.error_number, 1);
        assert_eq!(error_code.message, "Test error.");
        assert_eq!(error_code.details, None);
        assert_eq!(error_code.is_internal, false);
    }

    #[test]
    fn test_with_details() {
        let error_code = ErrorCode::new_internal_error(
            "TEST".to_string(),
            1,
            "Test error.".to_string(),
            false,
        )
        .unwrap()
        .with_details("Test details.".to_string());

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.error_number, 1);
        assert_eq!(error_code.message, "Test error.");
        assert_eq!(error_code.details, Some("Test details.".to_string()));
        assert_eq!(error_code.is_internal, true);
    }

    #[test]
    fn test_validate_prefix() {
        assert!(ErrorCode::validate_prefix("TEST").is_ok());
        assert!(ErrorCode::validate_prefix("test").is_err());
        assert!(ErrorCode::validate_prefix("TE").is_ok());
        assert!(ErrorCode::validate_prefix("T").is_err());
        assert!(ErrorCode::validate_prefix("TEST1").is_err());
    }

    #[test]
    fn test_with_full_code_works() {
        for i in 0..31 {
            let error_code = ErrorCode::new_internal_error(
                "TEST".to_string(),
                i,
                "Test error.".to_string(),
                false,
            )
            .unwrap()
            .with_code();

            assert_eq!(error_code.code, Some(format!("TEST{:05}", i)));
        }
    }
}
