#![cfg(feature = "rhai")]

use myc_core::domain::dtos::callback::{
    CallbackContext, CallbackError, CallbackExecutor,
};
use rhai::{Dynamic, Engine, Map, Scope, AST};
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

            // Expose context variables
            scope.push("status_code", context.status_code as i64);
            scope.push("duration_ms", context.duration_ms as i64);
            scope.push("upstream_path", context.upstream_path);
            scope.push("downstream_url", context.downstream_url);
            scope.push("method", context.method);
            scope.push("timestamp", context.timestamp);

            // Headers as map
            let headers_map: Map = context
                .headers
                .into_iter()
                .map(|(k, v)| (k.into(), Dynamic::from(v)))
                .collect();
            scope.push("headers", headers_map);

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
