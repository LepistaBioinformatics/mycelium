use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How the service sender tag is exposed on outgoing service emails.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default,
)]
#[serde(rename_all = "camelCase")]
pub enum SenderTagMode {
    /// The tag travels in the `X-Mycelium-Source-Service` header only.
    #[default]
    Header,

    /// The tag is also appended to the sender address (`local+tag@domain`).
    Plus,
}

/// `[core.serviceNotifications]` — instance-level settings for the
/// downstream service email notifications.
///
/// ```toml
/// [core.serviceNotifications]
/// senderTagMode = "header"
/// maxConnectionStringTtlInDays = 90
/// retentionDays = 90
/// templatesDir = "service-templates"
/// ```
///
/// `templatesDir` is required only when a service enables notifications;
/// that cross-check runs against the services list in the API config.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "ServiceNotificationsConfigRaw")]
pub struct ServiceNotificationsConfig {
    pub sender_tag_mode: SenderTagMode,

    /// Upper bound for the expiration of a notification connection string.
    pub max_connection_string_ttl_in_days: u32,

    /// How long the dispatch trace rows are kept.
    pub retention_days: u32,

    /// Root of the service-owned templates, isolated from `TEMPLATES_DIR`.
    pub templates_dir: Option<PathBuf>,
}

/// Raw config, validated into `ServiceNotificationsConfig`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServiceNotificationsConfigRaw {
    #[serde(default)]
    sender_tag_mode: SenderTagMode,
    #[serde(default = "default_max_connection_string_ttl_in_days")]
    max_connection_string_ttl_in_days: u32,
    #[serde(default = "default_retention_days")]
    retention_days: u32,
    #[serde(default)]
    templates_dir: Option<PathBuf>,
}

fn default_max_connection_string_ttl_in_days() -> u32 {
    90
}

fn default_retention_days() -> u32 {
    90
}

impl Default for ServiceNotificationsConfig {
    fn default() -> Self {
        Self {
            sender_tag_mode: SenderTagMode::default(),
            max_connection_string_ttl_in_days:
                default_max_connection_string_ttl_in_days(),
            retention_days: default_retention_days(),
            templates_dir: None,
        }
    }
}

impl TryFrom<ServiceNotificationsConfigRaw> for ServiceNotificationsConfig {
    type Error = String;

    fn try_from(
        config: ServiceNotificationsConfigRaw,
    ) -> Result<Self, Self::Error> {
        if config.max_connection_string_ttl_in_days == 0 {
            return Err(
                "serviceNotifications.maxConnectionStringTtlInDays must be \
                 greater than zero"
                    .into(),
            );
        }

        if config.retention_days == 0 {
            return Err(
                "serviceNotifications.retentionDays must be greater than zero"
                    .into(),
            );
        }

        Ok(Self {
            sender_tag_mode: config.sender_tag_mode,
            max_connection_string_ttl_in_days: config
                .max_connection_string_ttl_in_days,
            retention_days: config.retention_days,
            templates_dir: config.templates_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wrapper {
        #[serde(default)]
        service_notifications: ServiceNotificationsConfig,
    }

    #[test]
    fn defaults_apply_when_block_absent() {
        let wrapper: Wrapper = toml::from_str("").unwrap();

        assert_eq!(
            wrapper.service_notifications,
            ServiceNotificationsConfig::default()
        );
        assert_eq!(
            wrapper.service_notifications.sender_tag_mode,
            SenderTagMode::Header
        );
        assert_eq!(
            wrapper
                .service_notifications
                .max_connection_string_ttl_in_days,
            90
        );
        assert_eq!(wrapper.service_notifications.retention_days, 90);
        assert!(wrapper.service_notifications.templates_dir.is_none());
    }

    #[test]
    fn defaults_apply_when_block_empty() {
        let config: ServiceNotificationsConfig = toml::from_str("").unwrap();

        assert_eq!(config, ServiceNotificationsConfig::default());
    }

    #[test]
    fn parses_explicit_values() {
        let config: ServiceNotificationsConfig = toml::from_str(
            r#"
            senderTagMode = "plus"
            maxConnectionStringTtlInDays = 30
            retentionDays = 7
            templatesDir = "service-templates"
            "#,
        )
        .unwrap();

        assert_eq!(config.sender_tag_mode, SenderTagMode::Plus);
        assert_eq!(config.max_connection_string_ttl_in_days, 30);
        assert_eq!(config.retention_days, 7);
        assert_eq!(
            config.templates_dir,
            Some(PathBuf::from("service-templates"))
        );
    }

    #[test]
    fn rejects_unknown_sender_tag_mode() {
        let result = toml::from_str::<ServiceNotificationsConfig>(
            r#"senderTagMode = "x""#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_zero_max_ttl() {
        let result = toml::from_str::<ServiceNotificationsConfig>(
            "maxConnectionStringTtlInDays = 0",
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_zero_retention() {
        let result =
            toml::from_str::<ServiceNotificationsConfig>("retentionDays = 0");

        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_key() {
        let result =
            toml::from_str::<ServiceNotificationsConfig>("unknownKey = 1");

        assert!(result.is_err());
    }
}
