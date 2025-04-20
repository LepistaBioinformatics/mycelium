use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

/// The health status of the service
///
/// The status should be Unknown, Healthy or Unhealthy. At the startup of the
/// service, the status should be Unknown. When the health check is successful,
/// the status should be Healthy. When the health check is not successful, the
/// status should be Unhealthy.
///
#[derive(
    Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    /// The health status is unknown
    ///
    Unknown,

    /// The health status is healthy
    ///
    #[serde(rename_all = "camelCase")]
    Healthy {
        /// The timestamp for the last health check
        ///
        checked_at: DateTime<Local>,
    },

    /// The health status is fully unhealthy
    ///
    #[serde(rename_all = "camelCase")]
    Unhealthy {
        /// The timestamp for the last health check
        ///
        checked_at: DateTime<Local>,

        /// The number of attempts with unhealthy status
        ///
        attempts: u32,

        /// Unhealthy instances
        ///
        unhealthy_instances: Vec<UnhealthyInstance>,
    },

    #[serde(rename_all = "camelCase")]
    Unavailable {
        /// The timestamp for the last health check
        ///
        checked_at: DateTime<Local>,

        /// The number of attempts with unavailable status
        ///
        attempts: u32,

        /// The error message
        ///
        error_message: String,
    },
}

impl HealthStatus {
    pub fn set_health(checked_at: DateTime<Local>) -> Self {
        Self::Healthy { checked_at }
    }

    pub fn set_unhealthy(
        old_status: Self,
        checked_at: DateTime<Local>,
        attempts: u32,
        unhealthy_instance: UnhealthyInstance,
        max_instances: u32,
    ) -> Self {
        let (unhealthy_instances, attempts_new) = if let Self::Unhealthy {
            checked_at: _,
            attempts: attempts_old,
            mut unhealthy_instances,
        } = old_status
        {
            //
            // Insert the new unhealthy instance at the end of the vector
            //
            unhealthy_instances.insert(0, unhealthy_instance);
            (unhealthy_instances, attempts_old + attempts)
        } else {
            (vec![unhealthy_instance], attempts)
        };

        Self::Unhealthy {
            checked_at,
            attempts: attempts_new,
            unhealthy_instances: unhealthy_instances
                .into_iter()
                .take(max_instances as usize)
                .collect(),
        }
    }

    pub fn set_unavailable(
        checked_at: DateTime<Local>,
        attempts: u32,
        error_message: String,
    ) -> Self {
        Self::Unavailable {
            checked_at,
            attempts,
            error_message,
        }
    }
}

/// The unhealthy instance
///
/// The unhealthy instance is a single instance of the service that is
/// unhealthy.
///
#[derive(
    Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct UnhealthyInstance {
    /// The instance ID
    ///
    pub host: String,

    /// The instance status code
    ///
    pub status_code: u16,

    /// The instance response body
    ///
    pub response_body: Option<String>,

    /// The error message
    ///
    pub error_message: Option<String>,

    /// The timestamp for the last health check
    ///
    pub checked_at: DateTime<Local>,
}

#[derive(
    Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckInfo {
    /// Service ID
    ///
    /// The user provided or service name-based identifier.
    ///
    pub service_id: Uuid,

    /// Service name
    ///
    /// The user provided value.
    ///
    pub service_name: String,

    /// Timestamp
    ///
    /// The timestamp of the health check.
    ///
    pub checked_at: DateTime<Local>,

    /// Status code
    ///
    /// The status code of the health check.
    ///
    pub status_code: u16,

    /// Is service healthy
    ///
    /// Whether the service is healthy.
    ///
    pub is_service_healthy: bool,

    /// Error message
    ///
    /// The error message of the health check. Only present if the service is
    /// unavailable and unable to generate a valid HTTP response.
    ///
    pub error_message: Option<String>,

    /// Response time
    ///
    /// The response time of the health check.
    ///
    pub response_time_ms: u64,

    /// DNS resolved IP
    ///
    /// The IP address of the DNS resolved.
    ///
    pub dns_resolved_ip: String,

    /// Response body
    ///
    /// Required if the status code is not 2xx. The response body of the health
    /// check.
    ///
    pub response_body: Option<String>,

    /// Headers
    ///
    /// Required if the status code is not 2xx. The headers of the health check.
    ///
    pub headers: Option<HashMap<String, String>>,

    /// Content type
    ///
    /// Required if the status code is not 2xx. The content type of the health
    /// check.
    ///
    pub content_type: Option<String>,

    /// Response size bytes
    ///
    /// Required if the status code is not 2xx. The size of the response body.
    ///
    pub response_size_bytes: Option<u64>,

    /// Retry count
    ///
    /// Required if the status code is not 2xx. The number of times the health
    /// check has been retried.
    ///
    pub retry_count: Option<u32>,

    /// Whether the health check timed out
    ///
    /// Required if the status code is not 2xx. Whether the health check timed
    /// out.
    ///
    pub timeout_occurred: Option<bool>,
}

impl HealthCheckInfo {
    pub fn new_when_health(
        service_id: Uuid,
        service_name: String,
        status_code: u16,
        response_time_ms: u64,
        dns_resolved_ip: String,
    ) -> Self {
        Self {
            //
            // Auto-generated field
            //
            checked_at: Local::now(),
            is_service_healthy: true,

            //
            // Required fields when the health check is successful
            //
            service_id,
            service_name,
            status_code,
            response_time_ms,
            dns_resolved_ip,

            //
            // Optional fields when the health check is successful
            //
            response_body: None,
            headers: None,
            content_type: None,
            response_size_bytes: None,
            retry_count: None,
            timeout_occurred: None,
            error_message: None,
        }
    }

    /// This function should be used when the health check is not successful
    ///
    /// The response should include all fields of the health check info object.
    ///
    pub fn new_when_unhealthy(
        service_id: Uuid,
        service_name: String,
        status_code: u16,
        response_time_ms: u64,
        dns_resolved_ip: String,
        response_body: String,
        headers: HashMap<String, String>,
        content_type: String,
        response_size_bytes: u64,
        retry_count: u32,
        timeout_occurred: bool,
    ) -> Self {
        Self {
            //
            // Auto-generated field
            //
            checked_at: Local::now(),
            is_service_healthy: false,

            //
            // Required fields always present
            //
            service_id,
            service_name,
            status_code,
            response_time_ms,
            dns_resolved_ip,

            //
            // Required fields when the health check is not successful
            //
            response_body: Some(response_body),
            headers: Some(headers),
            content_type: Some(content_type),
            response_size_bytes: Some(response_size_bytes),
            retry_count: Some(retry_count),
            timeout_occurred: Some(timeout_occurred),
            error_message: None,
        }
    }

    pub fn new_when_unavailable(
        service_id: Uuid,
        service_name: String,
        error_message: String,
    ) -> Self {
        Self {
            //
            // Auto-generated field
            //
            checked_at: Local::now(),
            is_service_healthy: false,

            //
            // Required fields always present
            //
            service_id,
            service_name,
            status_code: 0,
            response_time_ms: 0,
            dns_resolved_ip: String::new(),

            //
            // Optional fields always present
            //
            response_body: None,
            headers: None,
            content_type: None,
            response_size_bytes: None,
            retry_count: None,
            timeout_occurred: None,
            error_message: Some(error_message),
        }
    }
}
