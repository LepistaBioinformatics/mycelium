use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

const SENDER_TAG_PATTERN: &str = r"^[a-z0-9-]{1,32}$";

const DISPLAY_NAME_FORBIDDEN_CHARS: [char; 5] = ['<', '>', '"', '\r', '\n'];

/// Who a service may send notification emails to.
#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub enum RecipientPolicy {
    /// Only the principal owner of the issuing account.
    Owner,

    /// Members (guests or owners) of the connection string tenant.
    TenantMembers,

    /// Addresses whose domain is listed in `allowedDomains`.
    Domains,

    /// Any address.
    Any,
}

/// Per-service opt-in for downstream email notifications.
///
/// An absent block is deny-by-default: the service can not obtain
/// notification connection strings nor send emails.
///
/// ```toml
/// [service-01.notifications]
/// senderTag = "service-01"
/// displayName = "Service 01"
/// issuerRoles = ["service-01-admin"]
/// issuerTenants = []
/// recipientPolicy = "owner"
/// allowedDomains = []
/// templates = ["invoice-ready"]
/// quotaPerHour = 100
/// maxRecipientsPerMessage = 10
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "ServiceNotificationConfigRaw")]
pub struct ServiceNotificationConfig {
    /// Unique tag identifying the service as an email sender
    /// (`^[a-z0-9-]{1,32}$`).
    pub sender_tag: String,

    /// Name shown on the `From` header (`"{displayName} via ..."`).
    pub display_name: String,

    /// Roles the issuer must hold to create notification strings.
    pub issuer_roles: Vec<String>,

    /// Tenants allowed to issue notification strings. Empty means any.
    pub issuer_tenants: Vec<Uuid>,

    pub recipient_policy: RecipientPolicy,

    /// Required and non-empty when `recipientPolicy = "domains"`.
    pub allowed_domains: Vec<String>,

    /// Template names the service may send.
    pub templates: Vec<String>,

    pub quota_per_hour: u32,

    pub max_recipients_per_message: u32,
}

/// Raw config, validated into `ServiceNotificationConfig`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServiceNotificationConfigRaw {
    sender_tag: String,
    display_name: String,
    issuer_roles: Vec<String>,
    #[serde(default)]
    issuer_tenants: Vec<Uuid>,
    recipient_policy: RecipientPolicy,
    #[serde(default)]
    allowed_domains: Vec<String>,
    templates: Vec<String>,
    #[serde(default = "default_quota_per_hour")]
    quota_per_hour: u32,
    #[serde(default = "default_max_recipients_per_message")]
    max_recipients_per_message: u32,
}

fn default_quota_per_hour() -> u32 {
    100
}

fn default_max_recipients_per_message() -> u32 {
    10
}

impl TryFrom<ServiceNotificationConfigRaw> for ServiceNotificationConfig {
    type Error = String;

    fn try_from(
        config: ServiceNotificationConfigRaw,
    ) -> Result<Self, Self::Error> {
        validate_sender_tag(&config.sender_tag)?;
        validate_display_name(&config.display_name)?;
        validate_lists(&config)?;
        validate_quotas(&config)?;

        Ok(Self {
            sender_tag: config.sender_tag,
            display_name: config.display_name,
            issuer_roles: config.issuer_roles,
            issuer_tenants: config.issuer_tenants,
            recipient_policy: config.recipient_policy,
            allowed_domains: config.allowed_domains,
            templates: config.templates,
            quota_per_hour: config.quota_per_hour,
            max_recipients_per_message: config.max_recipients_per_message,
        })
    }
}

pub(crate) fn validate_sender_tag(sender_tag: &str) -> Result<(), String> {
    let pattern = Regex::new(SENDER_TAG_PATTERN).map_err(|e| e.to_string())?;

    if !pattern.is_match(sender_tag) {
        return Err(format!(
            "notifications.senderTag must match {SENDER_TAG_PATTERN}, got \
             {sender_tag}"
        ));
    }

    Ok(())
}

fn validate_display_name(display_name: &str) -> Result<(), String> {
    if display_name.contains(DISPLAY_NAME_FORBIDDEN_CHARS) {
        return Err(
            "notifications.displayName must not contain <, >, \", CR or LF"
                .into(),
        );
    }

    Ok(())
}

fn validate_lists(config: &ServiceNotificationConfigRaw) -> Result<(), String> {
    if config.issuer_roles.is_empty() {
        return Err("notifications.issuerRoles must not be empty".into());
    }

    if config.recipient_policy == RecipientPolicy::Domains
        && config.allowed_domains.is_empty()
    {
        return Err("notifications.allowedDomains must not be empty when \
             recipientPolicy is domains"
            .into());
    }

    if config.templates.is_empty() {
        return Err("notifications.templates must not be empty".into());
    }

    Ok(())
}

fn validate_quotas(
    config: &ServiceNotificationConfigRaw,
) -> Result<(), String> {
    if config.quota_per_hour == 0 {
        return Err(
            "notifications.quotaPerHour must be greater than zero".into()
        );
    }

    if config.max_recipients_per_message == 0 {
        return Err(
            "notifications.maxRecipientsPerMessage must be greater than zero"
                .into(),
        );
    }

    if config.max_recipients_per_message > config.quota_per_hour {
        return Err("notifications.maxRecipientsPerMessage must not exceed \
             quotaPerHour: such a message could never be sent"
            .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
        senderTag = "service-01"
        displayName = "Service 01"
        issuerRoles = ["service-01-admin"]
        recipientPolicy = "owner"
        templates = ["invoice-ready"]
    "#;

    fn parse(toml: &str) -> Result<ServiceNotificationConfig, toml::de::Error> {
        toml::from_str::<ServiceNotificationConfig>(toml)
    }

    fn with_override(key: &str, value: &str) -> String {
        let mut lines: Vec<String> = VALID
            .lines()
            .filter(|line| !line.trim_start().starts_with(key))
            .map(str::to_string)
            .collect();

        lines.push(format!("{key} = {value}"));
        lines.join("\n")
    }

    #[test]
    fn parses_minimal_block_with_defaults() {
        let config = parse(VALID).unwrap();

        assert_eq!(config.sender_tag, "service-01");
        assert_eq!(config.display_name, "Service 01");
        assert_eq!(config.issuer_roles, vec!["service-01-admin".to_string()]);
        assert!(config.issuer_tenants.is_empty());
        assert_eq!(config.recipient_policy, RecipientPolicy::Owner);
        assert!(config.allowed_domains.is_empty());
        assert_eq!(config.templates, vec!["invoice-ready".to_string()]);
        assert_eq!(config.quota_per_hour, 100);
        assert_eq!(config.max_recipients_per_message, 10);
    }

    #[test]
    fn parses_every_recipient_policy() {
        for policy in ["owner", "tenantMembers", "any"] {
            let toml =
                with_override("recipientPolicy", &format!("\"{policy}\""));

            assert!(parse(&toml).is_ok(), "policy {policy}");
        }
    }

    #[test]
    fn rejects_sender_tag_outside_pattern() {
        for tag in ["", "Service01", "service_01", "service=01", "service;01"] {
            let toml = with_override("senderTag", &format!("\"{tag}\""));

            assert!(parse(&toml).is_err(), "tag {tag}");
        }

        let too_long = "a".repeat(33);
        let toml = with_override("senderTag", &format!("\"{too_long}\""));

        assert!(parse(&toml).is_err());
    }

    #[test]
    fn rejects_display_name_with_forbidden_chars() {
        for name in ["<b>", "a>b", "a\\\"b", "a\\rb", "a\\nb"] {
            let toml = with_override("displayName", &format!("\"{name}\""));

            assert!(parse(&toml).is_err(), "name {name}");
        }
    }

    #[test]
    fn rejects_empty_issuer_roles() {
        assert!(parse(&with_override("issuerRoles", "[]")).is_err());
    }

    #[test]
    fn rejects_domains_policy_without_allowed_domains() {
        let toml = with_override("recipientPolicy", "\"domains\"");

        assert!(parse(&toml).is_err());

        let toml = format!("{toml}\nallowedDomains = [\"example.com\"]");

        assert!(parse(&toml).is_ok());
    }

    #[test]
    fn rejects_empty_templates() {
        assert!(parse(&with_override("templates", "[]")).is_err());
    }

    #[test]
    fn rejects_zero_quotas() {
        assert!(parse(&with_override("quotaPerHour", "0")).is_err());
        assert!(parse(&with_override("maxRecipientsPerMessage", "0")).is_err());
    }

    #[test]
    fn rejects_more_recipients_per_message_than_the_hourly_quota() {
        // `maxRecipientsPerMessage` defaults to 10.
        assert!(parse(&with_override("quotaPerHour", "9")).is_err());
        assert!(parse(&with_override("quotaPerHour", "10")).is_ok());
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(parse(&with_override("replyTo", "\"a@b.com\"")).is_err());
    }

    #[test]
    fn json_round_trip_keeps_config() {
        let config = parse(VALID).unwrap();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: ServiceNotificationConfig =
            serde_json::from_str(&json).unwrap();

        assert_eq!(config, decoded);
    }
}
