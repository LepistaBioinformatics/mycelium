use serde::{Deserialize, Serialize};

/// This file should contains only the error code options. The error code options
/// are the error codes that are used in the application.
///
use super::NativeErrorCodes;

/// A temporary struct to represent the error codes options
///
/// This struct is used to represent the error codes options. The error codes
/// options are the error codes that are used in the application. The error
/// codes options are used to generate the error codes enum and the error codes.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TempNativeErrorCodesOptions {
    pub code: NativeErrorCodes,
    pub message: String,
    pub details: Option<String>,
    pub is_internal: bool,
}

/// Here error codes sources resides
///
/// This is a list of all the error codes that are used in the system.
const MESSAGES_SOURCE: &'static str = r#"[
    {
        "code": "MYC00001",
        "message": "Prisma Client Unavailable error",
        "details": "Prisma Client error. Could not fetch client.",
        "is_internal": true
    },
    {
        "code": "MYC00002",
        "message": "User already registered in Mycelium",
        "details": "When a manager account try to register a new account and the account owner already exists this error should be returned.",
        "is_internal": false
    },
    {
        "code": "MYC00003",
        "message": "Account already registered in Mycelium",
        "details": "When a manager account try to register a new account and the account already exists this error should be returned.",
        "is_internal": false
    },
    {
        "code": "MYC00004",
        "message": "Could not check profile verbose status",
        "details": "This error should be dispatched when use-cases could not access the account verbose-status during validations.",
        "is_internal": false
    },
    {
        "code": "MYC00005",
        "message": "Action restricted to active users",
        "details": "Indicates that the desired action should only be performed by active users.",
        "is_internal": true
    },
    {
        "code": "MYC00006",
        "message": "Action restricted to manager users",
        "details": "Indicates that the action requires manager privileges.",
        "is_internal": true
    },
    {
        "code": "MYC00007",
        "message": "Updating action failed",
        "details": "Action dispatched when an update action was preceded by unknown error.",
        "is_internal": true
    }
]"#;

pub(super) fn get_error_code_maps(
    exp_length: Option<usize>,
) -> Vec<TempNativeErrorCodesOptions> {
    let parsed_messages = match serde_json::from_str::<
        Vec<TempNativeErrorCodesOptions>,
    >(MESSAGES_SOURCE)
    {
        Ok(parsed) => parsed,
        Err(err) => panic!("Error parsing error codes: {}", err),
    };

    if parsed_messages.iter().map(|i| i.code).len() != parsed_messages.len() {
        panic!("Error parsing error codes: duplicated error codes");
    }

    if exp_length.is_some() {
        assert_eq!(parsed_messages.len(), exp_length.unwrap());
    }

    parsed_messages
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_error_code_options() {
        let error_code_options = get_error_code_maps(None);
        assert_eq!(error_code_options.len(), 7);
    }
}
