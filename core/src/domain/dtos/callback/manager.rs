use super::{CallbackContext, CallbackExecutor, ExecutionMode};

use std::sync::Arc;

pub struct CallbackManager {
    executors: Vec<Arc<dyn CallbackExecutor>>,
    mode: ExecutionMode,
}

impl CallbackManager {
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            executors: Vec::new(),
            mode,
        }
    }

    pub fn register(&mut self, callback: Arc<dyn CallbackExecutor>) {
        self.executors.push(callback);
    }

    pub async fn execute_all(&self, context: &CallbackContext) {
        match self.mode {
            ExecutionMode::Parallel => {
                let futures: Vec<_> = self
                    .executors
                    .iter()
                    .map(|callback| {
                        let _callback = Arc::clone(callback);
                        let ctx = context.clone();

                        tokio::spawn(async move {
                            if let Err(e) = _callback.execute(&ctx).await {
                                tracing::error!(
                                    "Callback {} failed: {e}",
                                    _callback.name(),
                                );
                            }
                        })
                    })
                    .collect();

                for future in futures {
                    let _ = future.await;
                }
            }
            ExecutionMode::Sequential => {
                for callback in &self.executors {
                    if let Err(e) = callback.execute(context).await {
                        tracing::error!(
                            "Callback {} failed: {e}",
                            callback.name(),
                        );
                    }
                }
            }
            ExecutionMode::FireAndForget => {
                for callback in &self.executors {
                    let _callback = Arc::clone(callback);
                    let ctx = context.clone();

                    tokio::spawn(async move {
                        if let Err(e) = _callback.execute(&ctx).await {
                            tracing::error!(
                                "Callback {} failed: {e}",
                                _callback.name(),
                            );
                        }
                    });
                }
            }
        }
    }
}
