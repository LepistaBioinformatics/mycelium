#![cfg(feature = "rhai")]

use myc_core::domain::dtos::callback::{
    CallbackContext, CallbackError, CallbackExecutor,
};
use rhai::{Dynamic, Engine, Map, Scope, AST};
use serde_json::Value as JsonValue;
use shaku::Component;
use std::sync::Arc;
use tonic::async_trait;

#[derive(Component)]
#[shaku(interface = CallbackExecutor)]
pub struct RhaiCallback {
    engine: Arc<Engine>,
    script: AST,
    name: String,
}

impl RhaiCallback {
    pub fn new(
        script_content: &str,
        name: String,
    ) -> Result<Self, CallbackError> {
        let mut engine = Engine::new();

        // Register helper functions
        engine.register_fn("log_info", |msg: &str| {
            tracing::info!("{}", msg);
        });

        engine.register_fn("log_warn", |msg: &str| {
            tracing::warn!("{}", msg);
        });

        engine.register_fn("log_error", |msg: &str| {
            tracing::error!("{}", msg);
        });

        let script = engine
            .compile(script_content)
            .map_err(|e| CallbackError::ScriptError(e.to_string()))?;

        Ok(Self {
            engine: Arc::new(engine),
            script,
            name,
        })
    }
}

#[async_trait]
impl CallbackExecutor for RhaiCallback {
    async fn execute(
        &self,
        context: &CallbackContext,
    ) -> Result<(), CallbackError> {
        let engine = Arc::clone(&self.engine);
        let script = self.script.clone();
        let context = context.clone();

        tokio::task::spawn_blocking(move || -> Result<(), CallbackError> {
            let mut scope = Scope::new();

            // Convert context to JSON value to preserve exact field names
            let context_json = serde_json::to_value(&context).map_err(|e| {
                CallbackError::ScriptError(format!(
                    "Failed to serialize context: {}",
                    e
                ))
            })?;

            // Convert JSON value to Rhai Dynamic and expose all fields
            if let JsonValue::Object(map) = context_json {
                for (key, value) in map {
                    let rhai_value = json_value_to_rhai_dynamic(value);
                    scope.push(key, rhai_value);
                }
            }

            // Also expose response_headers as array for iteration (backward compatibility)
            let headers_array: rhai::Array = context
                .response_headers
                .iter()
                .map(|(k, v)| {
                    let mut header_obj = Map::new();
                    header_obj.insert("key".into(), Dynamic::from(k.clone()));
                    header_obj.insert("value".into(), Dynamic::from(v.clone()));
                    Dynamic::from(header_obj)
                })
                .collect();

            scope.push("headers_array", headers_array);

            engine
                .run_ast_with_scope(&mut scope, &script)
                .map_err(|e| CallbackError::ScriptError(e.to_string()))
        })
        .await
        .map_err(|e| CallbackError::ScriptError(e.to_string()))?
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Convert a serde_json::Value to a Rhai Dynamic value
fn json_value_to_rhai_dynamic(value: JsonValue) -> Dynamic {
    match value {
        JsonValue::Null => Dynamic::UNIT,
        JsonValue::Bool(b) => Dynamic::from(b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::from(n.to_string())
            }
        }
        JsonValue::String(s) => Dynamic::from(s),
        JsonValue::Array(arr) => {
            let rhai_array: rhai::Array =
                arr.into_iter().map(json_value_to_rhai_dynamic).collect();
            Dynamic::from(rhai_array)
        }
        JsonValue::Object(obj) => {
            let mut rhai_map = Map::new();
            for (k, v) in obj {
                rhai_map.insert(k.into(), json_value_to_rhai_dynamic(v));
            }
            Dynamic::from(rhai_map)
        }
    }
}
