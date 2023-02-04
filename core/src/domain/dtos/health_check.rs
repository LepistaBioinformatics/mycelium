use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckConfig {
    pub path: String,
    pub health_response_codes: Vec<i32>,
}
