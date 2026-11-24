use crate::domain::dtos::{
    error_code::ErrorCode, error_code_filters::ErrorCodeFilters,
    native_error_codes::NativeErrorCodes,
};

use mycelium_base::utils::errors::MappedErrors;

/// List the in-code native error codes matching the filters
///
/// Natives are sorted by `(prefix, error_number)`.
pub fn list_matching_native_error_codes(
    filters: &ErrorCodeFilters,
) -> Result<Vec<ErrorCode>, MappedErrors> {
    let mut natives: Vec<ErrorCode> = NativeErrorCodes::to_error_codes()?
        .into_values()
        .filter(|native| filters.matches(native))
        .collect();

    natives.sort_by(|left, right| {
        (&left.prefix, left.error_number)
            .cmp(&(&right.prefix, right.error_number))
    });

    Ok(natives)
}

/// Find the in-code native error code with this exact `(prefix, code)`
pub fn find_native_error_code(
    prefix: &str,
    code: i32,
) -> Result<Option<ErrorCode>, MappedErrors> {
    let native = NativeErrorCodes::to_error_codes()?
        .into_values()
        .find(|native| native.prefix == prefix && native.error_number == code);

    Ok(native)
}

/// Whether the prefix is used by an in-code native error code
pub fn is_native_error_code_prefix(prefix: &str) -> Result<bool, MappedErrors> {
    let is_native_prefix = NativeErrorCodes::to_error_codes()?
        .into_values()
        .any(|native| native.prefix == prefix);

    Ok(is_native_prefix)
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn filters(
        prefix: Option<&str>,
        code: Option<i32>,
        is_internal: Option<bool>,
    ) -> ErrorCodeFilters {
        ErrorCodeFilters {
            prefix: prefix.map(str::to_string),
            code,
            is_internal,
        }
    }

    #[test]
    fn lists_every_native_sorted_without_filters() -> Result<(), MappedErrors> {
        let natives =
            list_matching_native_error_codes(&filters(None, None, None))?;

        assert_eq!(natives.len(), NativeErrorCodes::iter().count());
        assert!(natives.windows(2).all(|pair| {
            (&pair[0].prefix, pair[0].error_number)
                < (&pair[1].prefix, pair[1].error_number)
        }));

        Ok(())
    }

    #[test]
    fn matches_prefix_as_case_insensitive_substring() -> Result<(), MappedErrors>
    {
        let lower =
            list_matching_native_error_codes(&filters(Some("yc"), None, None))?;
        let unrelated = list_matching_native_error_codes(&filters(
            Some("SERVICE"),
            None,
            None,
        ))?;

        assert_eq!(lower.len(), NativeErrorCodes::iter().count());
        assert!(unrelated.is_empty());

        Ok(())
    }

    #[test]
    fn matches_code_exactly() -> Result<(), MappedErrors> {
        let natives =
            list_matching_native_error_codes(&filters(None, Some(23), None))?;

        assert_eq!(natives.len(), 1);
        assert_eq!(natives[0].error_number, 23);

        Ok(())
    }

    #[test]
    fn matches_is_internal_exactly() -> Result<(), MappedErrors> {
        let public = list_matching_native_error_codes(&filters(
            None,
            None,
            Some(false),
        ))?;
        let internal =
            list_matching_native_error_codes(&filters(None, None, Some(true)))?;

        assert!(public.iter().all(|native| !native.is_internal));
        assert!(internal.iter().all(|native| native.is_internal));
        assert_eq!(public.len(), 30);
        assert_eq!(internal.len(), 7);

        Ok(())
    }

    #[test]
    fn finds_native_by_exact_prefix_and_code() -> Result<(), MappedErrors> {
        assert!(find_native_error_code("MYC", 1)?.is_some());
        assert!(find_native_error_code("myc", 1)?.is_none());
        assert!(find_native_error_code("MYC", 24)?.is_none());
        assert!(is_native_error_code_prefix("MYC")?);
        assert!(!is_native_error_code_prefix("SVC")?);

        Ok(())
    }
}
