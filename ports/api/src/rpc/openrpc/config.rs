use crate::models::api_config::ApiConfig;

use myc_config::optional_config::OptionalConfig;

const RPC_PATH: &str = "_adm/rpc";
const ENV_OPENRPC_DEV_URL: &str = "MYCELIUM_OPENRPC_DEV_URL";
const ENV_OPENRPC_PROD_URL: &str = "MYCELIUM_OPENRPC_PROD_URL";

/// Resolved server URLs for the OpenRPC spec (config + env).
#[derive(Clone, Debug)]
pub struct OpenRpcSpecConfig {
    pub dev_url: String,
    pub prod_url: Option<String>,
}

impl Default for OpenRpcSpecConfig {
    fn default() -> Self {
        Self {
            dev_url: format!("http://localhost:8080/{}", RPC_PATH),
            prod_url: None,
        }
    }
}

impl OpenRpcSpecConfig {
    /// Build from ApiConfig; env vars override config file values.
    pub fn from_api_config(api: &ApiConfig) -> Self {
        let dev_url = std::env::var(ENV_OPENRPC_DEV_URL)
            .ok()
            .or_else(|| api.openrpc_dev_url.clone())
            .unwrap_or_else(|| {
                let scheme = if matches!(api.tls, OptionalConfig::Enabled(_)) {
                    "https"
                } else {
                    "http"
                };
                format!(
                    "{}://{}:{}/{}",
                    scheme, api.service_ip, api.service_port, RPC_PATH
                )
            });

        let prod_url = std::env::var(ENV_OPENRPC_PROD_URL)
            .ok()
            .or_else(|| api.openrpc_prod_url.clone());

        Self { dev_url, prod_url }
    }
}
