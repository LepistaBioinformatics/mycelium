mod callback_filters;
mod callback_type;
mod context;
mod error;
mod execution_mode;
mod manager;
mod response;

pub use callback_filters::*;
pub use callback_type::*;
pub use context::*;
pub use error::*;
pub use execution_mode::*;
pub use manager::*;
pub use response::*;

use crate::domain::dtos::http::HttpMethod;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallbackDefinition {
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

    /// The HTTP methods to filter http executions
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
    pub http_methods: Option<HashMap<CallbackStatement, Vec<HttpMethod>>>,

    /// The HTTP status codes to filter http executions
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
    pub http_status_codes: Option<HashMap<CallbackStatement, Vec<u16>>>,

    /// The HTTP headers to filter http executions
    ///
    /// If headers are not provided, the callback will be executed all times
    /// which the downstream response is returned. If headers are provided, the
    /// callback will only be executed if the downstream response contains the
    /// specified headers.
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "X-Custom-Header": "custom-value",
    ///     "X-Custom-Header-2": "custom-value-2",
    /// }
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

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
}

fn default_timeout() -> u64 {
    5000
}
