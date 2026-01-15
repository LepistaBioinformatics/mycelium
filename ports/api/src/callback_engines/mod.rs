mod http_engine;
mod javascript_engine;
mod python_engine;

pub(crate) use http_engine::*;
pub(crate) use javascript_engine::*;
pub(crate) use python_engine::*;

// -----------------------------------------------------------------------------
// OPTIONAL: RHAI ENGINE
// -----------------------------------------------------------------------------
#[cfg(feature = "rhai")]
mod rhai_engine;
#[cfg(feature = "rhai")]
pub(crate) use rhai_engine::*;

// -----------------------------------------------------------------------------
// HELPER: Convert Callbacks to CallbackExecutors
// -----------------------------------------------------------------------------
use myc_core::domain::dtos::{
    callback::{Callback, CallbackError, CallbackExecutor, CallbackType},
    http::HttpMethod,
};
use std::sync::Arc;

/// Create a callback engine from a callback configuration
///
/// This function creates a callback engine from a callback configuration.
///
/// Returns a callback executor.
///
/// Errors:
/// - CallbackError: If the callback configuration is invalid.
///
/// Example:
///
/// ```rust
/// let callback = Callback::new("my_callback", CallbackType::Http, "http://example.com");
/// let engine = create_engine_from_callback(&callback)?;
/// ```
///
pub(crate) fn create_engine_from_callback(
    callback: &Callback,
) -> Result<Arc<dyn CallbackExecutor>, CallbackError> {
    match callback.callback_type {
        CallbackType::Http => {
            let url = callback.url.clone().ok_or_else(|| {
                CallbackError::ConfigError(format!(
                    "HTTP callback '{}' missing URL",
                    callback.name
                ))
            })?;

            let config = http_engine::HttpCallbackConfig {
                url,
                method: callback
                    .method
                    .clone()
                    .unwrap_or(HttpMethod::Post)
                    .to_string(),
                timeout_ms: callback.timeout_ms,
                retry_count: callback.retry_count,
                retry_interval_ms: callback.retry_interval_ms,
            };

            Ok(Arc::new(HttpCallback::new(config)))
        }
        CallbackType::Javascript => {
            let script_path =
                callback.script_path.clone().ok_or_else(|| {
                    CallbackError::ConfigError(format!(
                        "JavaScript callback '{}' missing script_path",
                        callback.name
                    ))
                })?;

            let engine = JavaScriptCallback::new(
                script_path,
                callback.node_path.clone(),
                Some(callback.timeout_ms),
            )?;

            Ok(Arc::new(engine))
        }
        CallbackType::Python => {
            let script_path =
                callback.script_path.clone().ok_or_else(|| {
                    CallbackError::ConfigError(format!(
                        "Python callback '{}' missing script_path",
                        callback.name
                    ))
                })?;

            let engine = PythonCallback::new(
                script_path,
                callback.python_path.clone(),
                Some(callback.timeout_ms),
            )?;

            Ok(Arc::new(engine))
        }
        #[cfg(feature = "rhai")]
        CallbackType::Rhai => {
            let script = callback.script.clone().ok_or_else(|| {
                CallbackError::ConfigError(format!(
                    "Rhai callback '{}' missing script",
                    callback.name
                ))
            })?;

            let engine = RhaiCallback::new(&script, callback.name.clone())?;
            Ok(Arc::new(engine))
        }
        #[cfg(not(feature = "rhai"))]
        CallbackType::Rhai => Err(CallbackError::ConfigError(
            "Rhai callbacks are not supported (feature not enabled)"
                .to_string(),
        )),
    }
}
