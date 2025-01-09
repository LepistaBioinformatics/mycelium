use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(
    Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckConfig {
    pub path: String,
    pub health_response_codes: Vec<i32>,
}
