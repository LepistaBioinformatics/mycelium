use std::sync::Arc;

use super::{CallbackContext, CallbackResponse, ExecutionMode};

pub struct CallbackManager {
    callbacks: Vec<Arc<dyn CallbackResponse>>,
    execution_mode: ExecutionMode,
}

impl CallbackManager {
    pub fn new(execution_mode: ExecutionMode) -> Self {
        Self {
            callbacks: Vec::new(),
            execution_mode,
        }
    }

    pub fn register(&mut self, callback: Arc<dyn CallbackResponse>) {
        self.callbacks.push(callback);
    }

    pub async fn execute_all(&self, context: &CallbackContext) {
        match self.execution_mode {
            ExecutionMode::Parallel => {
                let futures: Vec<_> = self
                    .callbacks
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
                for callback in &self.callbacks {
                    if let Err(e) = callback.execute(context).await {
                        tracing::error!(
                            "Callback {} failed: {e}",
                            callback.name(),
                        );
                    }
                }
            }
            ExecutionMode::FireAndForget => {
                for callback in &self.callbacks {
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
