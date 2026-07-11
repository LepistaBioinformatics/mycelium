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
    }
}
