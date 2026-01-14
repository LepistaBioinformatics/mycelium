use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallbackContext {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub duration_ms: u64,
    pub upstream_path: String,
    pub downstream_url: String,
    pub method: String,
    pub timestamp: String,
    pub request_id: Option<String>,
    pub client_ip: Option<String>,
}
