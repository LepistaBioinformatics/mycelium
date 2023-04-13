/// This file should contains only the error code options. The error code options
/// are the error codes that are used in the application.
///
use super::NativeErrorCodes::{self, *};

pub(super) fn get_error_code_maps(
    exp_length: Option<usize>,
) -> Vec<(NativeErrorCodes, (String, Option<String>, bool))> {
    let sources = vec![
        (
            MYC00001,
            (
                "Prisma Client Unavailable error".to_string(),
                Some(
                    "Prisma Client error. Could not fetch client.".to_string(),
                ),
                true,
            ),
        ),
        (
            MYC00002,
            (
                "User already registered in Mycelium".to_string(),
                Some(
                    "When a manager account try to register a new account and 
the account owner already exists this error should be returned."
                        .to_string(),
                ),
                false,
            ),
        ),
        (
            MYC00003,
            (
                "Account already registered in Mycelium".to_string(),
                Some(
                    "When a manager account try to register a new account and 
the account already exists this error should be returned."
                        .to_string(),
                ),
                false,
            ),
        ),
        (
            MYC00004,
            (
                "Could not check profile verbose status".to_string(),
                Some(
                    "This error should be dispatched when use-cases could not 
access the account verbose-status during validations."
                        .to_string(),
                ),
                false,
            ),
        ),
        (
            MYC00005,
            (
                "Action restricted to active users".to_string(),
                Some(
                    "Indicates that the desired action should only be performed 
by active users."
                        .to_string(),
                ),
                true,
            ),
        ),
    ];

    if exp_length.is_some() {
        assert_eq!(sources.len(), exp_length.unwrap());
    }

    sources
}
