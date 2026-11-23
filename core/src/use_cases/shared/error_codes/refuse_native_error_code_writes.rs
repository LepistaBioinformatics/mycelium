use super::{find_native_error_code, is_native_error_code_prefix};
use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Refuse a database write that would act on an in-code native error code
pub fn refuse_native_error_code_write(
    prefix: &str,
    code: i32,
) -> Result<(), MappedErrors> {
    if find_native_error_code(prefix, code)?.is_none() {
        return Ok(());
    }

    use_case_err(format!(
        "Error code {prefix}{code:05} is native: native error codes are defined in code and cannot be changed"
    ))
    .with_code(NativeErrorCodes::MYC00018)
    .with_exp_true()
    .as_error()
}

/// Refuse registering a custom error code under a native prefix
///
/// Custom codes do not choose their number, so a custom code under a native
/// prefix could collide with a native `(prefix, code)`.
pub fn refuse_native_error_code_prefix(
    prefix: &str,
) -> Result<(), MappedErrors> {
    if !is_native_error_code_prefix(prefix)? {
        return Ok(());
    }

    use_case_err(format!(
        "Prefix {prefix} is reserved for native error codes, which are defined in code"
    ))
    .with_code(NativeErrorCodes::MYC00018)
    .with_exp_true()
    .as_error()
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_write_on_native_as_expected_error() {
        let error = refuse_native_error_code_write("MYC", 18)
            .expect_err("native writes must be refused");

        assert!(error.expected());
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }

    #[test]
    fn allows_write_on_custom_code() {
        assert!(refuse_native_error_code_write("SVC", 18).is_ok());
        assert!(refuse_native_error_code_write("MYC", 999).is_ok());
    }

    #[test]
    fn refuses_native_prefix_only() {
        assert!(refuse_native_error_code_prefix("MYC").is_err());
        assert!(refuse_native_error_code_prefix("SVC").is_ok());
    }
}
