use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::healthcheck_logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct HealthcheckInfo {
    pub route_id: Uuid,
    pub route_name: String,
    pub checked_at: DateTime<Utc>,
    pub status_code: i32,
    pub response_time_ms: i32,
    pub dns_resolved_ip: Option<String>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub headers: Option<JsonValue>,
    pub content_type: Option<String>,
    pub response_size_bytes: Option<i32>,
    pub retry_count: Option<i32>,
    pub timeout_occurred: Option<bool>,
}
