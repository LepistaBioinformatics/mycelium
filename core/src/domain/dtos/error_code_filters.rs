use crate::domain::dtos::error_code::ErrorCode;

/// Filters shared by the native and the custom error code listings
///
/// `prefix` matches as a case-insensitive substring (the adapters' `ILIKE
/// %prefix%`), `code` matches the error number exactly and `is_internal`
/// matches exactly.
#[derive(Clone, Debug, Default)]
pub struct ErrorCodeFilters {
    pub prefix: Option<String>,
    pub code: Option<i32>,
    pub is_internal: Option<bool>,
}

impl ErrorCodeFilters {
    pub fn matches(&self, error_code: &ErrorCode) -> bool {
        self.matches_prefix(&error_code.prefix)
            && self.matches_code(error_code.error_number)
            && self.matches_is_internal(error_code.is_internal)
    }

    fn matches_prefix(&self, prefix: &str) -> bool {
        self.prefix.as_ref().map_or(true, |wanted| {
            prefix.to_lowercase().contains(&wanted.to_lowercase())
        })
    }

    fn matches_code(&self, error_number: i32) -> bool {
        self.code.map_or(true, |wanted| wanted == error_number)
    }

    fn matches_is_internal(&self, is_internal: bool) -> bool {
        self.is_internal
            .map_or(true, |wanted| wanted == is_internal)
    }
}
