mod native_error_options;

use self::native_error_options::get_error_code_maps;
use crate::domain::dtos::error_code::ErrorCode;

use enum_iterator::{all, Sequence};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

/// Here the mycelium native error codes are defined
///
/// This is a list of all the error codes that are used in the system.
#[derive(
    Debug, PartialEq, Sequence, Serialize, Deserialize, Hash, Eq, Clone, Copy,
)]
pub enum NativeErrorCodes {
    ///
    /// code: "MYC00001",
    /// message: "Prisma Client Unavailable error",
    /// details: "Prisma Client error. Could not fetch client.",
    /// is_internal: true,
    /// is_native: true
    ///
    MYC00001 = 1,

    ///
    /// code: "MYC00002",
    /// message: "User already registered in Mycelium",
    /// details: "When a manager account try to register a new account and the account owner already exists this error should be returned.",
    /// is_internal: false,
    /// is_native: true
    ///
    MYC00002,

    ///
    /// code: "MYC00003",
    /// message: "Account already registered in Mycelium",
    /// details: "When a manager account try to register a new account and the account already exists this error should be returned.",
    /// is_internal: false,
    /// is_native: true
    ///
    MYC00003,

    ///
    /// code: "MYC00004",
    /// message: "Could not check profile verbose status",
    /// details: "This error should be dispatched when use-cases could not access the account verbose-status during validations.",
    /// is_internal: false,
    /// is_native: true
    ///
    MYC00004,

    ///
    /// code: "MYC00005",
    /// message: "Action restricted to active users",
    /// details: "Indicates that the desired action should only be performed by active users.",
    /// is_internal: true,
    /// is_native: true
    ///
    MYC00005,

    ///
    /// code: "MYC00006",
    /// message: "Action restricted to manager users",
    /// details: "Indicates that the action requires manager privileges.",
    /// is_internal: true,
    /// is_native: true
    ///
    MYC00006,

    ///
    /// code: "MYC00007",
    /// message: "Updating action failed",
    /// details: "Action dispatched when an update action was preceded by unknown error.",
    /// is_internal: true,
    /// is_native: true
    ///
    MYC00007,

    ///
    /// code: "MYC00008",
    /// message: "Token not found or expired",
    /// details: "Indicates that the token was not found or has expired.",
    /// is_internal: false,
    /// is_native: true
    ///
    MYC00008,

    ///
    /// code: "MYC00009",
    /// message: "User not found",
    /// details: "Indicates that the user was not found.",
    /// is_internal: false,
    /// is_native: true
    ///
    MYC00009,
}

impl NativeErrorCodes {
    // ? -----------------------------------------------------------------------
    // ? PUBLIC INSTANCE METHODS
    // ? -----------------------------------------------------------------------

    /// Get parts of a single error code enumerator element
    ///
    /// This method will return a tuple with the prefix and the code of the
    /// enumerator element.
    pub fn parts(&self) -> (String, i32) {
        let pattern = Self::get_self_validation_pattern();
        let error_item = &self.to_string();

        if !pattern.is_match(error_item) {
            panic!("Invalid native error code enum format.");
        }

        let capture = pattern.captures(error_item).unwrap();

        (capture[1].to_string(), capture[2].parse::<i32>().unwrap())
    }

    /// Get the error code as a str.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MYC00001 => "MYC00001",
            Self::MYC00002 => "MYC00002",
            Self::MYC00003 => "MYC00003",
            Self::MYC00004 => "MYC00004",
            Self::MYC00005 => "MYC00005",
            Self::MYC00006 => "MYC00006",
            Self::MYC00007 => "MYC00007",
            Self::MYC00008 => "MYC00008",
            Self::MYC00009 => "MYC00009",
        }
    }

    // ? -----------------------------------------------------------------------
    // ? PUBLIC STATIC METHODS
    // ? -----------------------------------------------------------------------

    /// Get the error code options.
    ///
    /// This method will check if all entries in the enum are in the correct.
    /// Case yes, it will return a hashmap with the native enums codes as keys
    /// and the error code as values.
    ///
    /// This method should be used during the initialization of the application.
    ///
    pub fn to_error_codes(
    ) -> Result<HashMap<NativeErrorCodes, ErrorCode>, MappedErrors> {
        // Check if enum entries are in the correct format.
        Self::self_validate()?;

        // Get the expected length of the error code options. Should be
        // equivalent to NativeErrorCodes enum length (number of enum entries).
        let expected_length = Self::to_vec().len();

        let mut error_code_sources =
            HashMap::<NativeErrorCodes, ErrorCode>::new();

        for source in get_error_code_maps(Some(expected_length)).iter() {
            let (prefix, code) = source.code.parts();

            let mut error_code = ErrorCode::new(
                prefix,
                code,
                source.message.to_string(),
                source.is_internal,
                source.is_native,
            )?;

            if let Some(details) = source.details.to_owned() {
                error_code = error_code.with_details(details.to_string());
            }

            error_code_sources
                .insert(source.code.to_owned(), error_code.to_owned());
        }

        Ok(error_code_sources)
    }

    // ? -----------------------------------------------------------------------
    // ? PRIVATE STATIC METHODS
    // ? -----------------------------------------------------------------------

    /// Create a vector with all the error code options.
    fn to_vec() -> Vec<Self> {
        all::<NativeErrorCodes>().collect::<Vec<_>>()
    }

    /// Validate the format of the enum
    ///
    /// This method will panic if the enum format is invalid. This is done to
    /// ensure that the enum is always in the correct format. This is a private
    /// method that is only used internally during tests.
    fn self_validate() -> Result<Regex, MappedErrors> {
        let pattern = Self::get_self_validation_pattern();
        let error_codes = Self::to_vec();

        for error_code in error_codes {
            let error_item = &error_code.to_string();

            if !pattern.is_match(error_item) {
                return execution_err(
                    "Invalid native error code enum format.".to_string(),
                )
                .as_error();
            }
        }

        Ok(pattern)
    }

    /// Get the regex pattern for the enum format validation.
    fn get_self_validation_pattern() -> Regex {
        Regex::new(r"^([A-Z]{2,4})([0-9]+)$").unwrap()
    }
}

impl Display for NativeErrorCodes {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_validate_native_error_code_enum_format() {
        let pattern = NativeErrorCodes::get_self_validation_pattern();

        let error_codes = NativeErrorCodes::to_vec();

        for error_code in error_codes {
            let error_item = &error_code.to_string();

            assert!(pattern.is_match(error_item));
        }
    }

    #[test]
    fn should_fail_to_validate_native_error_code_enum_format() {
        let pattern = NativeErrorCodes::get_self_validation_pattern();

        let error_codes = NativeErrorCodes::to_vec();

        for error_code in error_codes {
            let error_item = &error_code.to_string();

            assert!(pattern.is_match(error_item));
        }
    }

    #[test]
    fn should_get_native_error_code_parts() {
        let error_code = NativeErrorCodes::MYC00001;
        let (prefix, code) = error_code.parts();

        assert_eq!(prefix, "MYC");
        assert_eq!(code, 1);
    }

    #[test]
    fn should_fail_to_get_native_error_code_parts() {
        let error_code = NativeErrorCodes::MYC00001;
        let (prefix, code) = error_code.parts();

        assert_eq!(prefix, "MYC");
        assert_ne!(code, 2);
    }

    #[test]
    fn test_self_validate() {
        let pattern = NativeErrorCodes::self_validate().unwrap();

        let error_codes = NativeErrorCodes::to_vec();

        for error_code in error_codes {
            let error_item = &error_code.to_string();

            assert!(pattern.is_match(error_item));
        }
    }
}
