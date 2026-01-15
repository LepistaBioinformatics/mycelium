mod callback_filters;
mod callback_type;
mod context;
mod error;
mod execution_mode;
mod executor;
mod manager;

pub use callback_filters::*;
pub use callback_type::*;
pub use context::*;
pub use error::*;
pub use execution_mode::*;
pub use executor::*;
pub use manager::*;

use crate::domain::dtos::http::HttpMethod;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Reason why a callback was blocked from execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackBlockReason {
    /// Blocked by HTTP method filter
    MethodFilter,
    /// Blocked by status code filter
    StatusCodeFilter,
    /// Blocked by header filter
    HeaderFilter,
}

impl fmt::Display for CallbackBlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallbackBlockReason::MethodFilter => {
                write!(f, "HTTP method filter")
            }
            CallbackBlockReason::StatusCodeFilter => {
                write!(f, "status code filter")
            }
            CallbackBlockReason::HeaderFilter => write!(f, "header filter"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Callback {
    // -------------------------------------------------------------------------
    // Common
    // -------------------------------------------------------------------------
    /// Callback name
    ///
    /// Use this name to specify the callback in routes configuration.
    ///
    /// Example:
    ///
    /// ```json
    /// "name": "my_callback"
    /// ```
    ///
    pub name: String,

    /// Callback type
    ///
    /// The type of callback to use. Allowed values are:
    /// - `rhai`
    /// - `javascript`
    /// - `python`
    /// - `http`
    ///
    #[serde(rename = "type")]
    pub callback_type: CallbackType,

    /// The timeout in milliseconds
    ///
    /// The maximum time to wait for the callback to complete. Ignores with
    /// execution mode is set to `FireAndForget`. Default is 5 seconds.
    ///
    /// Example:
    ///
    /// ```json
    /// 3600  # 1 hour
    /// ```
    ///
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// The number of times to retry the callback if it fails
    ///
    /// Default is 0.
    ///
    /// Example:
    ///
    /// ```json
    /// 3
    /// ```
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// The interval in milliseconds to retry the callback if it fails
    ///
    /// Default is 1000 milliseconds.
    ///
    /// Example:
    ///
    /// ```json
    /// 5000
    /// ```
    #[serde(default = "default_retry_interval_ms")]
    pub retry_interval_ms: u64,

    /// The HTTP methods to trigger the callback
    ///
    /// If methods are not provided, the callback will be executed for all
    /// methods. If methods are provided, the callback will only be executed for
    /// the specified methods.
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "OneOf": ["GET", "POST"],
    ///     "AllOf": ["GET", "POST", "PUT", "DELETE"],
    ///     "NoneOf": ["HEAD", "OPTIONS", "TRACE", "CONNECT"]
    /// }
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggering_methods: Option<HashMap<CallbackStatement, Vec<HttpMethod>>>,

    /// The HTTP status codes to trigger the callback
    ///
    /// If status codes are not provided, the callback will be executed for all
    /// status codes. If status codes are provided, the callback will only be
    /// executed for the specified status codes.
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "OneOf": [200, 201],
    ///     "AllOf": [200, 201, 202, 203],
    ///     "NoneOf": [400, 401, 402, 403]
    /// }
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggering_status_codes: Option<HashMap<CallbackStatement, Vec<u16>>>,

    /// The HTTP headers to trigger the callback
    ///
    /// If headers are not provided, the callback will be executed all times
    /// which the downstream response is returned. If headers are provided, the
    /// callback will only be executed if the downstream response contains the
    /// specified headers according to the statement type.
    ///
    /// - `OneOf`: At least one set of headers must be present in the response
    /// - `AllOf`: All sets of headers must be present in the response
    /// - `NoneOf`: None of the sets of headers must be present in the response
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "oneOf": {
    ///         "X-Custom-Header": "custom-value",
    ///         "X-Custom-Header-2": "custom-value-2"
    ///     },
    ///     "allOf": {
    ///         "Content-Type": "application/json"
    ///     },
    ///     "noneOf": {
    ///         "X-Error-Header": "error-value"
    ///     }
    /// }
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggering_headers:
        Option<HashMap<CallbackStatement, HashMap<String, String>>>,

    // -------------------------------------------------------------------------
    // Rhai specific
    // -------------------------------------------------------------------------
    /// The script to execute.
    ///
    /// This is only used for Rhai callbacks.
    ///
    /// Example:
    ///
    /// ```toml
    /// [[callbacks]]
    /// name = "performance_monitor"
    /// type = "rhai"
    /// script = """
    /// if status_code >= 500 {
    ///     log_error("Server error detected!");
    /// }
    /// if duration_ms > 1000 {
    ///     log_warn("Slow response detected!");
    /// }
    /// """
    /// timeout_ms = 1000  # 1 second
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,

    // -------------------------------------------------------------------------
    // JavaScript or Python specific
    // -------------------------------------------------------------------------
    /// The path to the script file
    ///
    /// Example:
    ///
    /// ```bash
    /// /path/to/script.js
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<PathBuf>,

    /// Python interpreter path
    ///
    /// The path to the Python interpreter
    ///
    /// Example:
    ///
    /// ```bash
    /// /usr/bin/python3.12
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_path: Option<String>,

    /// Node.js interpreter path
    ///
    /// Example:
    ///
    /// ```bash
    /// /usr/bin/node
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_path: Option<String>,

    // -------------------------------------------------------------------------
    // HTTP specific
    // -------------------------------------------------------------------------
    /// HTTP specific
    ///
    /// The URL to call
    ///
    /// Example:
    ///
    /// ```bash
    /// https://example.com
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// The HTTP method to use
    ///
    /// If the method is not provided, the default method is POST. Only write
    /// methods (POST, PUT, PATCH, DELETE) are allowed.
    ///
    #[serde(
        skip_serializing_if = "Option::is_none",
        default = "default_method"
    )]
    pub method: Option<HttpMethod>,
}

fn default_retry_count() -> u32 {
    3
}

fn default_retry_interval_ms() -> u64 {
    1000
}

fn default_timeout() -> u64 {
    5000
}

fn default_method() -> Option<HttpMethod> {
    Some(HttpMethod::Post)
}

impl Callback {
    /// Check if the callback should be executed based on the provided context
    ///
    /// This function evaluates all triggering filters (methods, status codes, headers)
    /// to determine if the callback should be executed.
    ///
    /// Returns `Ok(())` if the callback should be executed, or `Err(CallbackBlockReason)`
    /// indicating which filter blocked the execution.
    pub fn should_execute(
        &self,
        http_method: &HttpMethod,
        status_code: u16,
        response_headers: &HashMap<String, String>,
    ) -> Result<(), CallbackBlockReason> {
        // Check triggering methods
        if let Some(ref methods) = self.triggering_methods {
            if !Self::check_method_filters(methods, http_method) {
                return Err(CallbackBlockReason::MethodFilter);
            }
        }

        // Check triggering status codes
        if let Some(ref status_codes) = self.triggering_status_codes {
            if !Self::check_status_code_filters(status_codes, status_code) {
                return Err(CallbackBlockReason::StatusCodeFilter);
            }
        }

        // Check triggering headers
        if let Some(ref headers) = self.triggering_headers {
            if !Self::check_header_filters(headers, response_headers) {
                return Err(CallbackBlockReason::HeaderFilter);
            }
        }

        Ok(())
    }

    /// Check if the HTTP method matches the triggering methods filter
    fn check_method_filters(
        filters: &HashMap<CallbackStatement, Vec<HttpMethod>>,
        method: &HttpMethod,
    ) -> bool {
        let mut one_of_match = false;
        let mut all_of_match = true;
        let mut none_of_match = true;

        for (statement, methods) in filters {
            match statement {
                CallbackStatement::OneOf => {
                    if methods.contains(method) {
                        one_of_match = true;
                    }
                }
                CallbackStatement::AllOf => {
                    if !methods.contains(method) {
                        all_of_match = false;
                    }
                }
                CallbackStatement::NoneOf => {
                    if methods.contains(method) {
                        none_of_match = false;
                    }
                }
            }
        }

        // If OneOf is specified, at least one must match
        // If AllOf is specified, all must match
        // If NoneOf is specified, none must match
        let has_one_of = filters.contains_key(&CallbackStatement::OneOf);
        let has_all_of = filters.contains_key(&CallbackStatement::AllOf);
        let has_none_of = filters.contains_key(&CallbackStatement::NoneOf);

        if has_one_of && !one_of_match {
            return false;
        }
        if has_all_of && !all_of_match {
            return false;
        }
        if has_none_of && !none_of_match {
            return false;
        }

        true
    }

    /// Check if the status code matches the triggering status codes filter
    fn check_status_code_filters(
        filters: &HashMap<CallbackStatement, Vec<u16>>,
        status_code: u16,
    ) -> bool {
        let mut one_of_match = false;
        let mut all_of_match = true;
        let mut none_of_match = true;

        for (statement, codes) in filters {
            match statement {
                CallbackStatement::OneOf => {
                    if codes.contains(&status_code) {
                        one_of_match = true;
                    }
                }
                CallbackStatement::AllOf => {
                    if !codes.contains(&status_code) {
                        all_of_match = false;
                    }
                }
                CallbackStatement::NoneOf => {
                    if codes.contains(&status_code) {
                        none_of_match = false;
                    }
                }
            }
        }

        let has_one_of = filters.contains_key(&CallbackStatement::OneOf);
        let has_all_of = filters.contains_key(&CallbackStatement::AllOf);
        let has_none_of = filters.contains_key(&CallbackStatement::NoneOf);

        if has_one_of && !one_of_match {
            return false;
        }
        if has_all_of && !all_of_match {
            return false;
        }
        if has_none_of && !none_of_match {
            return false;
        }

        true
    }

    /// Check if the response headers match the triggering headers filter
    ///
    /// This function verifies that header filters match by checking the
    /// complete key-value pairs. A header is considered a match only if both
    /// the key AND value match exactly. Partial matches (key only or value
    /// only) are not considered valid matches.
    ///
    /// Note: Header names are compared case-insensitively (as per HTTP spec),
    /// but values are compared case-sensitively.
    fn check_header_filters(
        filters: &HashMap<CallbackStatement, HashMap<String, String>>,
        response_headers: &HashMap<String, String>,
    ) -> bool {
        // Helper function to check if a set of headers is present in response
        fn headers_match(
            required_headers: &HashMap<String, String>,
            response_headers: &HashMap<String, String>,
        ) -> bool {
            // Verify each required header as a complete key-value pair
            for (required_key, required_value) in required_headers {
                // Find matching header key (case-insensitive comparison)
                // HTTP header names are case-insensitive per RFC 7230
                let matching_key = response_headers
                    .keys()
                    .find(|key| key.eq_ignore_ascii_case(required_key));

                // Get the actual value from response headers for this key
                let actual_value = match matching_key {
                    Some(key) => response_headers.get(key).unwrap(),
                    // Key doesn't exist (case-insensitive) - no match
                    None => {
                        tracing::trace!(
                            "Header '{}' not found in response headers",
                            required_key
                        );
                        return false;
                    }
                };

                // Both key (case-insensitive) and value (case-sensitive) must match
                if actual_value != required_value {
                    tracing::trace!(
                        "Header '{}' value mismatch: expected '{}', got '{}'",
                        required_key,
                        required_value,
                        actual_value
                    );
                    return false;
                }
            }
            true
        }

        let mut one_of_match = false;
        let mut all_of_match = true;
        let mut none_of_match = true;

        for (statement, headers) in filters {
            let matches = headers_match(headers, response_headers);
            match statement {
                CallbackStatement::OneOf => {
                    if matches {
                        one_of_match = true;
                    }
                }
                CallbackStatement::AllOf => {
                    if !matches {
                        all_of_match = false;
                    }
                }
                CallbackStatement::NoneOf => {
                    if matches {
                        none_of_match = false;
                    }
                }
            }
        }

        let has_one_of = filters.contains_key(&CallbackStatement::OneOf);
        let has_all_of = filters.contains_key(&CallbackStatement::AllOf);
        let has_none_of = filters.contains_key(&CallbackStatement::NoneOf);

        if has_one_of && !one_of_match {
            return false;
        }
        if has_all_of && !all_of_match {
            return false;
        }
        if has_none_of && !none_of_match {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_callback() -> Callback {
        Callback {
            name: "test_callback".to_string(),
            callback_type: CallbackType::Http,
            timeout_ms: 5000,
            retry_count: 0,
            retry_interval_ms: 1000,
            triggering_methods: None,
            triggering_status_codes: None,
            triggering_headers: None,
            script: None,
            script_path: None,
            python_path: None,
            node_path: None,
            url: Some("http://example.com".to_string()),
            method: None,
        }
    }

    fn create_test_headers() -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        headers
    }

    #[test]
    fn test_should_execute_without_filters() {
        let callback = create_test_callback();
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Post, 404, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Put, 500, &headers)
            .is_ok());
    }

    #[test]
    fn test_should_execute_with_method_filter_oneof() {
        let mut callback = create_test_callback();
        let mut methods = HashMap::new();
        methods.insert(
            CallbackStatement::OneOf,
            vec![HttpMethod::Get, HttpMethod::Post],
        );
        callback.triggering_methods = Some(methods);
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Post, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Put, 200, &headers)
            .is_err());
        assert!(callback
            .should_execute(&HttpMethod::Delete, 200, &headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_method_filter_allof() {
        let mut callback = create_test_callback();
        let mut methods = HashMap::new();
        methods.insert(
            CallbackStatement::AllOf,
            vec![HttpMethod::Get, HttpMethod::Post, HttpMethod::Put],
        );
        callback.triggering_methods = Some(methods);
        let headers = create_test_headers();

        // AllOf requires the method to be in the list
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Post, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Put, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Delete, 200, &headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_method_filter_noneof() {
        let mut callback = create_test_callback();
        let mut methods = HashMap::new();
        methods.insert(
            CallbackStatement::NoneOf,
            vec![HttpMethod::Delete, HttpMethod::Patch],
        );
        callback.triggering_methods = Some(methods);
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Post, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Delete, 200, &headers)
            .is_err());
        assert!(callback
            .should_execute(&HttpMethod::Patch, 200, &headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_status_code_filter_oneof() {
        let mut callback = create_test_callback();
        let mut status_codes = HashMap::new();
        status_codes.insert(CallbackStatement::OneOf, vec![200, 201, 404]);
        callback.triggering_status_codes = Some(status_codes);
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 201, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 404, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 500, &headers)
            .is_err());
        assert!(callback
            .should_execute(&HttpMethod::Get, 301, &headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_status_code_filter_allof() {
        let mut callback = create_test_callback();
        let mut status_codes = HashMap::new();
        status_codes.insert(CallbackStatement::AllOf, vec![200, 201, 202]);
        callback.triggering_status_codes = Some(status_codes);
        let headers = create_test_headers();

        // AllOf requires the status code to be in the list
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 201, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 202, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 404, &headers)
            .is_err());
        assert!(callback
            .should_execute(&HttpMethod::Get, 500, &headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_status_code_filter_noneof() {
        let mut callback = create_test_callback();
        let mut status_codes = HashMap::new();
        status_codes.insert(CallbackStatement::NoneOf, vec![400, 401, 500]);
        callback.triggering_status_codes = Some(status_codes);
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 201, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Get, 400, &headers)
            .is_err());
        assert!(callback
            .should_execute(&HttpMethod::Get, 401, &headers)
            .is_err());
        assert!(callback
            .should_execute(&HttpMethod::Get, 500, &headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_header_filter_oneof() {
        let mut callback = create_test_callback();
        let mut header_filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        required_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        header_filters.insert(CallbackStatement::OneOf, required_headers);
        callback.triggering_headers = Some(header_filters);
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());

        // Missing header
        let mut incomplete_headers = HashMap::new();
        incomplete_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &incomplete_headers)
            .is_err());

        // Wrong value
        let mut wrong_headers = HashMap::new();
        wrong_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        wrong_headers
            .insert("X-Custom-Header".to_string(), "wrong-value".to_string());
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &wrong_headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_header_filter_allof() {
        let mut callback = create_test_callback();
        let mut header_filters = HashMap::new();

        // First set of headers
        let mut headers1 = HashMap::new();
        headers1
            .insert("Content-Type".to_string(), "application/json".to_string());

        // Second set of headers
        let mut headers2 = HashMap::new();
        headers2
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());

        header_filters.insert(CallbackStatement::AllOf, headers1.clone());
        // Note: AllOf with multiple sets means all sets must match
        // For simplicity, we'll test with one set that contains multiple headers
        let mut all_headers = HashMap::new();
        all_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        all_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        header_filters.clear();
        header_filters.insert(CallbackStatement::AllOf, all_headers);

        callback.triggering_headers = Some(header_filters);
        let headers = create_test_headers();

        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());

        // Missing one header
        let mut incomplete_headers = HashMap::new();
        incomplete_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &incomplete_headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_header_filter_noneof() {
        let mut callback = create_test_callback();
        let mut header_filters = HashMap::new();
        let mut forbidden_headers = HashMap::new();
        forbidden_headers
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        header_filters.insert(CallbackStatement::NoneOf, forbidden_headers);
        callback.triggering_headers = Some(header_filters);
        let headers = create_test_headers();

        // Headers don't contain the forbidden header - should pass
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());

        // Headers contain the forbidden header - should fail
        let mut error_headers = HashMap::new();
        error_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        error_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        error_headers
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &error_headers)
            .is_err());
    }

    #[test]
    fn test_should_execute_with_multiple_filters() {
        let mut callback = create_test_callback();

        // Method filter: OneOf GET, POST
        let mut methods = HashMap::new();
        methods.insert(
            CallbackStatement::OneOf,
            vec![HttpMethod::Get, HttpMethod::Post],
        );
        callback.triggering_methods = Some(methods);

        // Status code filter: OneOf 200, 201
        let mut status_codes = HashMap::new();
        status_codes.insert(CallbackStatement::OneOf, vec![200, 201]);
        callback.triggering_status_codes = Some(status_codes);

        // Header filter: Content-Type must be application/json
        let mut header_filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        header_filters.insert(CallbackStatement::OneOf, required_headers);
        callback.triggering_headers = Some(header_filters);

        let headers = create_test_headers();

        // All filters pass
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &headers)
            .is_ok());
        assert!(callback
            .should_execute(&HttpMethod::Post, 201, &headers)
            .is_ok());

        // Method filter fails
        assert!(callback
            .should_execute(&HttpMethod::Put, 200, &headers)
            .is_err());

        // Status code filter fails
        assert!(callback
            .should_execute(&HttpMethod::Get, 404, &headers)
            .is_err());

        // Header filter fails
        let mut wrong_headers = HashMap::new();
        wrong_headers
            .insert("Content-Type".to_string(), "text/html".to_string());
        assert!(callback
            .should_execute(&HttpMethod::Get, 200, &wrong_headers)
            .is_err());
    }

    #[test]
    fn test_check_method_filters_oneof() {
        let mut filters = HashMap::new();
        filters.insert(
            CallbackStatement::OneOf,
            vec![HttpMethod::Get, HttpMethod::Post],
        );

        assert!(Callback::check_method_filters(&filters, &HttpMethod::Get));
        assert!(Callback::check_method_filters(&filters, &HttpMethod::Post));
        assert!(!Callback::check_method_filters(&filters, &HttpMethod::Put));
        assert!(!Callback::check_method_filters(
            &filters,
            &HttpMethod::Delete
        ));
    }

    #[test]
    fn test_check_method_filters_allof() {
        let mut filters = HashMap::new();
        filters.insert(
            CallbackStatement::AllOf,
            vec![HttpMethod::Get, HttpMethod::Post, HttpMethod::Put],
        );

        assert!(Callback::check_method_filters(&filters, &HttpMethod::Get));
        assert!(Callback::check_method_filters(&filters, &HttpMethod::Post));
        assert!(Callback::check_method_filters(&filters, &HttpMethod::Put));
        assert!(!Callback::check_method_filters(
            &filters,
            &HttpMethod::Delete
        ));
    }

    #[test]
    fn test_check_method_filters_noneof() {
        let mut filters = HashMap::new();
        filters.insert(
            CallbackStatement::NoneOf,
            vec![HttpMethod::Delete, HttpMethod::Patch],
        );

        assert!(Callback::check_method_filters(&filters, &HttpMethod::Get));
        assert!(Callback::check_method_filters(&filters, &HttpMethod::Post));
        assert!(!Callback::check_method_filters(
            &filters,
            &HttpMethod::Delete
        ));
        assert!(!Callback::check_method_filters(
            &filters,
            &HttpMethod::Patch
        ));
    }

    #[test]
    fn test_check_method_filters_multiple_statements() {
        let mut filters = HashMap::new();
        filters.insert(
            CallbackStatement::OneOf,
            vec![HttpMethod::Get, HttpMethod::Post],
        );
        filters.insert(CallbackStatement::NoneOf, vec![HttpMethod::Delete]);

        // GET is in OneOf and not in NoneOf - should pass
        assert!(Callback::check_method_filters(&filters, &HttpMethod::Get));

        // DELETE is in NoneOf - should fail
        assert!(!Callback::check_method_filters(
            &filters,
            &HttpMethod::Delete
        ));

        // PUT is not in OneOf - should fail
        assert!(!Callback::check_method_filters(&filters, &HttpMethod::Put));
    }

    #[test]
    fn test_check_status_code_filters_oneof() {
        let mut filters = HashMap::new();
        filters.insert(CallbackStatement::OneOf, vec![200, 201, 404]);

        assert!(Callback::check_status_code_filters(&filters, 200));
        assert!(Callback::check_status_code_filters(&filters, 201));
        assert!(Callback::check_status_code_filters(&filters, 404));
        assert!(!Callback::check_status_code_filters(&filters, 500));
        assert!(!Callback::check_status_code_filters(&filters, 301));
    }

    #[test]
    fn test_check_status_code_filters_allof() {
        let mut filters = HashMap::new();
        filters.insert(CallbackStatement::AllOf, vec![200, 201, 202]);

        assert!(Callback::check_status_code_filters(&filters, 200));
        assert!(Callback::check_status_code_filters(&filters, 201));
        assert!(Callback::check_status_code_filters(&filters, 202));
        assert!(!Callback::check_status_code_filters(&filters, 404));
        assert!(!Callback::check_status_code_filters(&filters, 500));
    }

    #[test]
    fn test_check_status_code_filters_noneof() {
        let mut filters = HashMap::new();
        filters.insert(CallbackStatement::NoneOf, vec![400, 401, 500]);

        assert!(Callback::check_status_code_filters(&filters, 200));
        assert!(Callback::check_status_code_filters(&filters, 201));
        assert!(!Callback::check_status_code_filters(&filters, 400));
        assert!(!Callback::check_status_code_filters(&filters, 401));
        assert!(!Callback::check_status_code_filters(&filters, 500));
    }

    #[test]
    fn test_check_status_code_filters_multiple_statements() {
        let mut filters = HashMap::new();
        filters.insert(CallbackStatement::OneOf, vec![200, 201, 404]);
        filters.insert(CallbackStatement::NoneOf, vec![500]);

        // 200 is in OneOf and not in NoneOf - should pass
        assert!(Callback::check_status_code_filters(&filters, 200));

        // 500 is in NoneOf - should fail
        assert!(!Callback::check_status_code_filters(&filters, 500));

        // 301 is not in OneOf - should fail
        assert!(!Callback::check_status_code_filters(&filters, 301));
    }

    #[test]
    fn test_check_header_filters_oneof() {
        let mut filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        required_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        filters.insert(CallbackStatement::OneOf, required_headers);

        let mut response_headers = HashMap::new();
        response_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        response_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        response_headers
            .insert("X-Other-Header".to_string(), "other-value".to_string());

        assert!(Callback::check_header_filters(&filters, &response_headers));

        // Missing header
        let mut incomplete_headers = HashMap::new();
        incomplete_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        assert!(!Callback::check_header_filters(
            &filters,
            &incomplete_headers
        ));
    }

    #[test]
    fn test_check_header_filters_allof() {
        let mut filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        required_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        filters.insert(CallbackStatement::AllOf, required_headers);

        let mut response_headers = HashMap::new();
        response_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        response_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());

        assert!(Callback::check_header_filters(&filters, &response_headers));

        // Missing header
        let mut incomplete_headers = HashMap::new();
        incomplete_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        assert!(!Callback::check_header_filters(
            &filters,
            &incomplete_headers
        ));

        // Wrong value
        let mut wrong_headers = HashMap::new();
        wrong_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        wrong_headers
            .insert("X-Custom-Header".to_string(), "wrong-value".to_string());
        assert!(!Callback::check_header_filters(&filters, &wrong_headers));
    }

    #[test]
    fn test_check_header_filters_noneof() {
        let mut filters = HashMap::new();
        let mut forbidden_headers = HashMap::new();
        forbidden_headers
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        filters.insert(CallbackStatement::NoneOf, forbidden_headers);

        let mut response_headers = HashMap::new();
        response_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        response_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());

        // Headers don't contain the forbidden header - should pass
        assert!(Callback::check_header_filters(&filters, &response_headers));

        // Headers contain the forbidden header - should fail
        response_headers
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        assert!(!Callback::check_header_filters(&filters, &response_headers));
    }

    #[test]
    fn test_check_header_filters_multiple_statements() {
        let mut filters = HashMap::new();
        let mut headers1 = HashMap::new();
        headers1
            .insert("Content-Type".to_string(), "application/json".to_string());
        filters.insert(CallbackStatement::OneOf, headers1);

        let mut headers2 = HashMap::new();
        headers2
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        filters.insert(CallbackStatement::NoneOf, headers2);

        let mut response_headers = HashMap::new();
        response_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        response_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());

        // Content-Type matches OneOf and X-Error-Header is not present (NoneOf) - should pass
        assert!(Callback::check_header_filters(&filters, &response_headers));

        // X-Error-Header is present (NoneOf) - should fail
        response_headers
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        assert!(!Callback::check_header_filters(&filters, &response_headers));
    }

    #[test]
    fn test_check_header_filters_case_insensitive_names() {
        let mut filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        filters.insert(CallbackStatement::OneOf, required_headers);

        // Header names are case-insensitive per HTTP spec (RFC 7230)
        let mut response_headers_lowercase = HashMap::new();
        response_headers_lowercase
            .insert("content-type".to_string(), "application/json".to_string());
        assert!(Callback::check_header_filters(
            &filters,
            &response_headers_lowercase
        ));

        let mut response_headers_uppercase = HashMap::new();
        response_headers_uppercase
            .insert("CONTENT-TYPE".to_string(), "application/json".to_string());
        assert!(Callback::check_header_filters(
            &filters,
            &response_headers_uppercase
        ));

        let mut response_headers_mixed = HashMap::new();
        response_headers_mixed
            .insert("CoNtEnT-TyPe".to_string(), "application/json".to_string());
        assert!(Callback::check_header_filters(
            &filters,
            &response_headers_mixed
        ));
    }

    #[test]
    fn test_check_header_filters_case_sensitive_values() {
        let mut filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        filters.insert(CallbackStatement::OneOf, required_headers);

        // Header values are case-sensitive
        let mut response_headers_wrong_value = HashMap::new();
        response_headers_wrong_value
            .insert("Content-Type".to_string(), "APPLICATION/JSON".to_string());
        assert!(!Callback::check_header_filters(
            &filters,
            &response_headers_wrong_value
        ));

        // Correct value should match
        let mut response_headers_correct = HashMap::new();
        response_headers_correct
            .insert("Content-Type".to_string(), "application/json".to_string());
        assert!(Callback::check_header_filters(
            &filters,
            &response_headers_correct
        ));
    }

    #[test]
    fn test_check_header_filters_empty_required() {
        let filters = HashMap::new();
        let mut response_headers = HashMap::new();
        response_headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        // Empty filters should always pass
        assert!(Callback::check_header_filters(&filters, &response_headers));
    }

    #[test]
    fn test_check_header_filters_case_insensitive_allof() {
        let mut filters = HashMap::new();
        let mut required_headers = HashMap::new();
        required_headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        required_headers
            .insert("X-Custom-Header".to_string(), "custom-value".to_string());
        filters.insert(CallbackStatement::AllOf, required_headers);

        // All headers with different case in names should still match
        let mut response_headers = HashMap::new();
        response_headers
            .insert("content-type".to_string(), "application/json".to_string());
        response_headers
            .insert("x-custom-header".to_string(), "custom-value".to_string());
        assert!(Callback::check_header_filters(&filters, &response_headers));
    }

    #[test]
    fn test_check_header_filters_case_insensitive_noneof() {
        let mut filters = HashMap::new();
        let mut forbidden_headers = HashMap::new();
        forbidden_headers
            .insert("X-Error-Header".to_string(), "error-value".to_string());
        filters.insert(CallbackStatement::NoneOf, forbidden_headers);

        // Header with different case should still be detected as forbidden
        let mut response_headers = HashMap::new();
        response_headers
            .insert("x-error-header".to_string(), "error-value".to_string());
        assert!(!Callback::check_header_filters(&filters, &response_headers));

        // Header with different case in value should pass (value is case-sensitive)
        let mut response_headers_different_value = HashMap::new();
        response_headers_different_value
            .insert("x-error-header".to_string(), "ERROR-VALUE".to_string());
        assert!(Callback::check_header_filters(
            &filters,
            &response_headers_different_value
        ));
    }
}
