use myc_config::{load_config_from_file, secret_resolver::SecretResolver};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueConfig {
    #[serde(default = "default_email_queue_name")]
    pub email_queue_name: SecretResolver<String>,
    #[serde(default = "default_consume_interval_in_secs")]
    pub consume_interval_in_secs: SecretResolver<u64>,
    // Number of messages claimed per dispatcher tick. Kept small by default so
    // the multi-pod claim's visibility-timeout invariant holds (see below and
    // the diesel `list_oldest_messages` claim query). Bounds throughput to
    // `claim_batch_size` messages per successful tick.
    #[serde(default = "default_claim_batch_size")]
    pub claim_batch_size: SecretResolver<i32>,
    // How long a claimed message stays invisible to other pods (the
    // `FOR UPDATE SKIP LOCKED` claim stamps `attempted`). MUST exceed the
    // worst-case time to process a whole claimed batch, or a slow-but-live pod
    // double-sends. It ALSO doubles as the retry back-off for a failed send:
    // lower = faster retries but requires a smaller batch to stay safe.
    // Invariant: visibility_timeout_secs > claim_batch_size * worst_case_smtp_send.
    #[serde(default = "default_visibility_timeout_secs")]
    pub visibility_timeout_secs: SecretResolver<i64>,
}

fn default_email_queue_name() -> SecretResolver<String> {
    SecretResolver::Value("emails".to_string())
}

fn default_consume_interval_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(15)
}

fn default_claim_batch_size() -> SecretResolver<i32> {
    SecretResolver::Value(3)
}

fn default_visibility_timeout_secs() -> SecretResolver<i64> {
    SecretResolver::Value(240)
}

// Loaded on its own -- not bundled with `SmtpConfig` -- so standalone mode
// (which has no `[smtp]` section) can still load `[queue]`, the polling
// config the email dispatcher needs regardless of backend.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    queue: QueueConfig,
}

impl QueueConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<TmpConfig>(file) {
            Ok(config) => Ok(config.queue),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_config_defaults_when_fields_absent() {
        let config: QueueConfig = toml::from_str("").unwrap();

        assert_eq!(
            config.email_queue_name,
            SecretResolver::Value("emails".to_string())
        );
        assert_eq!(config.consume_interval_in_secs, SecretResolver::Value(15));
        assert_eq!(config.claim_batch_size, SecretResolver::Value(3));
        assert_eq!(config.visibility_timeout_secs, SecretResolver::Value(240));
    }
}
