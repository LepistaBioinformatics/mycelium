use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

/// This struct is used to manage the webhook configurations.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WebhookConfig {
    /// Consume interval in seconds
    #[serde(default = "default_consume_interval_in_secs")]
    pub consume_interval_in_secs: SecretResolver<u64>,

    /// Batch consume size
    #[serde(default = "default_consume_batch_size")]
    pub consume_batch_size: SecretResolver<u64>,

    /// Max attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: SecretResolver<u64>,

    /// Accept invalid certificates
    #[serde(default = "default_accept_invalid_certificates")]
    pub accept_invalid_certificates: SecretResolver<bool>,

    /// Whole-request timeout, in seconds
    ///
    /// Without it a downstream that accepts the connection and never answers
    /// holds the dispatcher's future open forever, and the claim window below
    /// has no worst case to be sized against.
    ///
    #[serde(default = "default_request_timeout_in_secs")]
    pub request_timeout_in_secs: SecretResolver<u64>,

    /// Connect-phase timeout, in seconds
    #[serde(default = "default_connect_timeout_in_secs")]
    pub connect_timeout_in_secs: SecretResolver<u64>,

    /// Delay before the first retry, in seconds
    ///
    /// Doubles with every further attempt, up to `retry_cap_in_secs`.
    ///
    #[serde(default = "default_retry_base_in_secs")]
    pub retry_base_in_secs: SecretResolver<u64>,

    /// Ceiling on the exponential retry delay, in seconds
    #[serde(default = "default_retry_cap_in_secs")]
    pub retry_cap_in_secs: SecretResolver<u64>,

    /// How long a claimed event stays un-reclaimable, in seconds
    ///
    /// INVARIANT: this must exceed the worst-case wall-clock of one whole
    /// claimed batch, because the batch is marked `processing` up front and
    /// then dispatched sequentially. The HTTP part of that bound is
    /// `consume_batch_size * request_timeout_in_secs` (25 x 30 = 750s at the
    /// defaults) and it is a floor, not the whole cost -- each event also pays
    /// a `list_by_trigger`, a KEK derivation, a DEK fetch and a sequential
    /// secret-decryption loop. Raising the batch or the request timeout
    /// REQUIRES raising this proportionally, or a live-but-slow pod has its
    /// un-dispatched rows reclaimed by another pod and double-sent.
    ///
    /// This is crash recovery only; it does not shape the retry spacing, which
    /// is what `retry_base_in_secs` is for.
    ///
    #[serde(default = "default_visibility_timeout_in_secs")]
    pub visibility_timeout_in_secs: SecretResolver<i64>,
}

fn default_consume_interval_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(30)
}

fn default_consume_batch_size() -> SecretResolver<u64> {
    SecretResolver::Value(25)
}

fn default_max_attempts() -> SecretResolver<u64> {
    SecretResolver::Value(5)
}

fn default_accept_invalid_certificates() -> SecretResolver<bool> {
    SecretResolver::Value(true)
}

fn default_request_timeout_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(30)
}

fn default_connect_timeout_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(10)
}

fn default_retry_base_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(30)
}

fn default_retry_cap_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(3600)
}

fn default_visibility_timeout_in_secs() -> SecretResolver<i64> {
    SecretResolver::Value(900)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_config_defaults_when_fields_absent() {
        let config: WebhookConfig = toml::from_str("").unwrap();

        assert_eq!(config.consume_interval_in_secs, SecretResolver::Value(30));
        assert_eq!(config.consume_batch_size, SecretResolver::Value(25));
        assert_eq!(config.max_attempts, SecretResolver::Value(5));
        assert_eq!(
            config.accept_invalid_certificates,
            SecretResolver::Value(true)
        );
        assert_eq!(config.request_timeout_in_secs, SecretResolver::Value(30));
        assert_eq!(config.connect_timeout_in_secs, SecretResolver::Value(10));
        assert_eq!(config.retry_base_in_secs, SecretResolver::Value(30));
        assert_eq!(config.retry_cap_in_secs, SecretResolver::Value(3600));
        assert_eq!(
            config.visibility_timeout_in_secs,
            SecretResolver::Value(900)
        );
    }

    #[test]
    fn visibility_window_default_covers_the_default_batch() {
        let config: WebhookConfig = toml::from_str("").unwrap();

        let (batch, timeout, window) = match (
            config.consume_batch_size,
            config.request_timeout_in_secs,
            config.visibility_timeout_in_secs,
        ) {
            (
                SecretResolver::Value(b),
                SecretResolver::Value(t),
                SecretResolver::Value(w),
            ) => (b, t, w),
            _ => panic!("defaults must be plain values"),
        };

        // The invariant documented on `visibility_timeout_in_secs`. Whoever
        // changes one of these three defaults has to change the others with it.
        assert!(
            window as u64 > batch * timeout,
            "visibility window {window}s does not cover {batch} x {timeout}s"
        );
    }
}
