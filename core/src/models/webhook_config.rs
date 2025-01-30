use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

/// This struct is used to manage the webhook configurations.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WebhookConfig {
    /// Consume interval in seconds
    pub consume_interval_in_secs: SecretResolver<u64>,

    /// Batch consume size
    pub consume_batch_size: SecretResolver<u64>,
}
