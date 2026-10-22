use super::http::HttpMethod;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;

const MAX_ATTEMPTS_LIMIT: u8 = 10;

/// Opt-in retry of a route's downstream request (issue #137).
///
/// ```toml
/// [[my-service.path]]
/// group = "authenticated"
/// path = "/items/*"
/// methods = ["GET", "PUT"]
/// retry = { maxAttempts = 3, baseDelayMs = 100, maxDelayMs = 2000 }
/// ```
///
/// Each attempt is bounded by `api.gatewayTimeout`, so the worst case for a
/// request is `maxAttempts * gatewayTimeout` plus the delays.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "RouteRetryPolicyConfig")]
pub struct RouteRetryPolicy {
    /// Attempts in total, the first one included (1 to 10).
    pub max_attempts: u8,

    /// Delay before the first retry; the n-th retry waits
    /// `baseDelayMs * n²`, capped at `maxDelayMs`.
    pub base_delay_ms: u64,

    pub max_delay_ms: u64,

    /// Downstream response statuses that trigger a retry (5xx or 429).
    pub retry_on_status: Vec<u16>,

    /// Also retry POST/PATCH/CONNECT, which may not be safe to repeat.
    /// Connection failures only; a request that may have reached the service
    /// is never repeated for these methods.
    pub retry_non_idempotent: bool,
}

/// Raw config, validated into `RouteRetryPolicy`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteRetryPolicyConfig {
    #[serde(default = "default_max_attempts")]
    max_attempts: u8,
    #[serde(default = "default_base_delay_ms")]
    base_delay_ms: u64,
    #[serde(default = "default_max_delay_ms")]
    max_delay_ms: u64,
    #[serde(default = "default_retry_on_status")]
    retry_on_status: Vec<u16>,
    #[serde(default)]
    retry_non_idempotent: bool,
}

fn default_max_attempts() -> u8 {
    3
}

fn default_base_delay_ms() -> u64 {
    100
}

fn default_max_delay_ms() -> u64 {
    2_000
}

fn default_retry_on_status() -> Vec<u16> {
    vec![502, 503, 504]
}

impl TryFrom<RouteRetryPolicyConfig> for RouteRetryPolicy {
    type Error = String;

    fn try_from(config: RouteRetryPolicyConfig) -> Result<Self, Self::Error> {
        if !(1..=MAX_ATTEMPTS_LIMIT).contains(&config.max_attempts) {
            return Err(format!(
                "retry.maxAttempts must be between 1 and {MAX_ATTEMPTS_LIMIT}"
            ));
        }

        if config.base_delay_ms > config.max_delay_ms {
            return Err(
                "retry.baseDelayMs must not exceed retry.maxDelayMs".into()
            );
        }

        if let Some(status) = config
            .retry_on_status
            .iter()
            .find(|status| !(**status == 429 || (500..=599).contains(*status)))
        {
            return Err(format!(
                "retry.retryOnStatus accepts 429 and 5xx statuses only, got \
                 {status}"
            ));
        }

        Ok(Self {
            max_attempts: config.max_attempts,
            base_delay_ms: config.base_delay_ms,
            max_delay_ms: config.max_delay_ms,
            retry_on_status: config.retry_on_status,
            retry_non_idempotent: config.retry_non_idempotent,
        })
    }
}

impl RouteRetryPolicy {
    /// Wait before the `retry`-th retry (1-based): quadratic back-off.
    pub fn delay_for(&self, retry: u32) -> Duration {
        let factor = u64::from(retry).saturating_mul(u64::from(retry));
        let delay = self.base_delay_ms.saturating_mul(factor);

        Duration::from_millis(delay.min(self.max_delay_ms))
    }

    pub fn retries_status(&self, status: u16) -> bool {
        self.retry_on_status.contains(&status)
    }

    /// Whether a request with this method may be sent more than once.
    pub fn applies_to(&self, method: &HttpMethod) -> bool {
        Self::is_idempotent(method) || self.retry_non_idempotent
    }

    /// Methods that are safe to repeat after the request may have reached
    /// the service (RFC 9110 §9.2.2).
    pub fn is_idempotent(method: &HttpMethod) -> bool {
        matches!(
            method,
            HttpMethod::Get
                | HttpMethod::Head
                | HttpMethod::Options
                | HttpMethod::Put
                | HttpMethod::Delete
                | HttpMethod::Trace
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(toml_value: &str) -> Result<RouteRetryPolicy, String> {
        #[derive(Deserialize)]
        struct Wrapper {
            retry: RouteRetryPolicy,
        }

        serde_json::from_str::<Wrapper>(toml_value)
            .map(|wrapper| wrapper.retry)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn defaults_apply_to_an_empty_table() {
        let policy = policy(r#"{ "retry": {} }"#).unwrap();

        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay_ms, 100);
        assert_eq!(policy.max_delay_ms, 2_000);
        assert_eq!(policy.retry_on_status, vec![502, 503, 504]);
        assert!(!policy.retry_non_idempotent);
    }

    #[test]
    fn delay_grows_quadratically_up_to_the_cap() {
        let policy = policy(
            r#"{ "retry": { "baseDelayMs": 100, "maxDelayMs": 1000 } }"#,
        )
        .unwrap();

        assert_eq!(policy.delay_for(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for(2), Duration::from_millis(400));
        assert_eq!(policy.delay_for(3), Duration::from_millis(900));
        assert_eq!(policy.delay_for(4), Duration::from_millis(1000));
    }

    #[test]
    fn only_idempotent_methods_unless_opted_in() {
        let default = policy(r#"{ "retry": {} }"#).unwrap();
        let opted_in =
            policy(r#"{ "retry": { "retryNonIdempotent": true } }"#).unwrap();

        assert!(default.applies_to(&HttpMethod::Get));
        assert!(default.applies_to(&HttpMethod::Put));
        assert!(!default.applies_to(&HttpMethod::Post));
        assert!(!default.applies_to(&HttpMethod::Patch));
        assert!(opted_in.applies_to(&HttpMethod::Post));
    }

    #[test]
    fn invalid_configs_are_rejected() {
        assert!(policy(r#"{ "retry": { "maxAttempts": 0 } }"#).is_err());
        assert!(policy(r#"{ "retry": { "maxAttempts": 11 } }"#).is_err());
        assert!(policy(
            r#"{ "retry": { "baseDelayMs": 500, "maxDelayMs": 100 } }"#
        )
        .is_err());
        assert!(policy(r#"{ "retry": { "retryOnStatus": [404] } }"#).is_err());
        assert!(
            policy(r#"{ "retry": { "retryOnStatus": [429, 500] } }"#).is_ok()
        );
    }
}
