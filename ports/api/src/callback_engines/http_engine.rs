use myc_core::domain::dtos::callback::{
    CallbackContext, CallbackError, CallbackResponse,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shaku::Component;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpCallbackConfig {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub retry_count: u32,
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Component)]
#[shaku(interface = CallbackResponse)]
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
impl CallbackResponse for HttpCallback {
    async fn execute(
        &self,
        context: &CallbackContext,
    ) -> Result<(), CallbackError> {
        let payload = serde_json::json!({
            "status_code": context.status_code,
            "headers": context.headers,
            "duration_ms": context.duration_ms,
            "upstream_path": context.upstream_path,
            "downstream_url": context.downstream_url,
            "method": context.method,
            "timestamp": context.timestamp,
            "request_id": context.request_id,
            "client_ip": context.client_ip,
        });

        let mut request = self
            .client
            .request(self.config.method.parse().unwrap(), &self.config.url)
            .json(&payload);

        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| CallbackError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CallbackError::HttpError(format!(
                "HTTP {} returned status {}",
                self.config.url,
                response.status()
            )));
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
