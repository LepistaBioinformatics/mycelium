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
    pub code: i32,

    /// The message of the error.
    pub message: String,

    /// The details of the error.
    pub details: Option<String>,

    /// Whether the error is internal or external.
    pub is_internal: bool,
}

impl ErrorCode {
    // ? -----------------------------------------------------------------------
    // ? PUBLIC STRUCTURAL METHODS (CONSTRUCTORS)
    // ? -----------------------------------------------------------------------

    /// Creates a new internal ErrorCode with the given code and message.
    pub fn new_internal_error(
        prefix: String,
        code: i32,
        message: String,
    ) -> Result<Self, String> {
        ErrorCode::validate_prefix(&prefix)?;

        Ok(Self {
            prefix,
            code,
            message,
            details: None,
            is_internal: true,
        })
    }

    /// Creates a new external ErrorCode with the given code and message.
    pub fn new_external_error(
        prefix: String,
        code: i32,
        message: String,
    ) -> Result<Self, String> {
        ErrorCode::validate_prefix(&prefix)?;

        Ok(Self {
            prefix,
            code,
            message,
            details: None,
            is_internal: false,
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

    // ? -----------------------------------------------------------------------
    // ? PRIVATE STATIC METHODS
    // ? -----------------------------------------------------------------------

    /// Validates the given prefix.
    fn validate_prefix(prefix: &str) -> Result<(), String> {
        let max_prefix_size = 5;
        let min_prefix_size = 1;

        if prefix.len() <= min_prefix_size {
            return Err(format!(
                "Prefix must be longest than {min_prefix_size}."
            ));
        }

        if prefix.len() >= max_prefix_size {
            return Err(format!(
                "Prefix must be shortest than {max_prefix_size}."
            ));
        }

        if !prefix
            .chars()
            .all(|c| c.is_ascii_alphabetic() && c.is_uppercase())
        {
            return Err("Prefix must be all alphabetic.".to_string());
        }

        Ok(())
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
        )
        .unwrap();

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.code, 1);
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
        )
        .unwrap();

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.code, 1);
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
        )
        .unwrap()
        .with_details("Test details.".to_string());

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.code, 1);
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
}
