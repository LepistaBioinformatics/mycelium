use myc_core::domain::dtos::callback::{
    CallbackContext, CallbackError, CallbackExecutor,
};
use shaku::Component;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Component)]
#[shaku(interface = CallbackExecutor)]
pub struct PythonCallback {
    script_path: PathBuf,
    python_path: String,
    timeout_ms: u64,
    name: String,
}

impl PythonCallback {
    pub fn new(
        script_path: impl Into<PathBuf>,
        python_path: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Result<Self, CallbackError> {
        let script_path = script_path.into();

        if !script_path.exists() {
            return Err(CallbackError::ConfigError(format!(
                "Script not found: {}",
                script_path.display()
            )));
        }

        let name = script_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(Self {
            script_path,
            python_path: python_path.unwrap_or_else(|| "python3".to_string()),
            timeout_ms: timeout_ms.unwrap_or(5000),
            name,
        })
    }
}

#[async_trait::async_trait]
impl CallbackExecutor for PythonCallback {
    async fn execute(
        &self,
        context: &CallbackContext,
    ) -> Result<(), CallbackError> {
        let context_json = serde_json::to_string(context).map_err(|e| {
            CallbackError::ScriptError(format!(
                "Failed to serialize context: {}",
                e
            ))
        })?;

        let output = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            Command::new(&self.python_path)
                .arg(&self.script_path)
                .arg("--context")
                .arg(&context_json)
                .output(),
        )
        .await
        .map_err(|_| {
            CallbackError::ScriptError("Script execution timeout".into())
        })?
        .map_err(|e| {
            CallbackError::ScriptError(format!(
                "Failed to execute python: {}",
                e
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CallbackError::ScriptError(format!(
                "Script failed: {}",
                stderr
            )));
        }

        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::debug!("Script output: {}", stdout);
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
