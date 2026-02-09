use myc_core::domain::dtos::callback::{
    CallbackContext, CallbackError, CallbackExecutor,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shaku::Component;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpCallbackConfig {
    pub url: String,
    pub method: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub retry_interval_ms: u64,
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Component)]
#[shaku(interface = CallbackExecutor)]
pub struct HttpCallback {
    config: HttpCallbackConfig,
    client: Client,
    name: String,
}

impl HttpCallback {
    pub fn new(config: HttpCallbackConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap();

        let name = format!("http_{}", config.url);

        Self {
            config,
            client,
            name,
        }
    }
}

#[async_trait::async_trait]
impl CallbackExecutor for HttpCallback {
    async fn execute(
        &self,
        context: &CallbackContext,
    ) -> Result<(), CallbackError> {
        let payload = serde_json::json!({
            "status_code": context.status_code,
            "headers": context.response_headers,
            "duration_ms": context.duration_ms,
            "upstream_path": context.upstream_path,
            "downstream_url": context.downstream_url,
            "method": context.method,
            "timestamp": context.timestamp,
            "request_id": context.request_id,
            "client_ip": context.client_ip,
        });

        // Total attempts = initial attempt + retry_count
        let total_attempts = self.config.retry_count + 1;
        let mut last_error: Option<CallbackError> = None;
        let mut current_interval = self.config.retry_interval_ms;

        for attempt in 0..total_attempts {
            // Try to send the request
            let result = self
                .client
                .request(self.config.method.parse().unwrap(), &self.config.url)
                .json(&payload)
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(());
                    } else {
                        last_error = Some(CallbackError::HttpError(format!(
                            "HTTP request returned status {}",
                            response.status()
                        )));
                    }
                }
                Err(e) => {
                    last_error = Some(CallbackError::HttpError(e.to_string()));
                }
            }

            // If this is not the last attempt, wait before retrying with exponential backoff
            if attempt < total_attempts - 1 {
                tracing::warn!(
                    "HTTP callback attempt {} failed, retrying in {}ms (exponential backoff)",
                    attempt + 1,
                    current_interval
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(
                    current_interval,
                ))
                .await;

                // Double the interval for next retry (exponential backoff)
                current_interval *= 2;
            }
        }

        // All attempts failed - return the last error
        Err(last_error.unwrap_or_else(|| {
            CallbackError::HttpError(
                "Unknown error during HTTP callback execution".to_string(),
            )
        }))
    }

    fn name(&self) -> &str {
        &self.name
    }
}
