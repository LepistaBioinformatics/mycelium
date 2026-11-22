use super::AccountLifeCycle;

use http::Uri;
use myc_config::load_config_from_file;
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `[core.federation]` — the instance federation settings
///
/// ```toml
/// [core.federation]
/// displayName = "Instance 01"
/// baseUrl = "https://myc-01.example.com"
/// assertionTtlSecs = 60
/// assertionRefreshSecs = 30
/// peerTimeoutMs = 800
/// resolveDeadlineMs = 1500
/// maxFailuresBeforeOpen = 5
/// linkRequestTtlSecs = 600
/// requestToleranceSecs = 30
/// ```
///
/// Only `baseUrl` is required. There is no key here: the federation key is
/// derived from `core.accountLifeCycle.hmacSecrets` (D-8). Whether federation
/// is on lives in `instance_settings`, not in this file (FED-06).
///
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "FederationConfigRaw")]
pub struct FederationConfig {
    /// Human-readable name published in the self-description
    pub display_name: Option<String>,

    /// Canonical public base URL of this instance, absolute http(s)
    pub base_url: String,

    /// Lifetime of an issued assertion (D-1)
    pub assertion_ttl_secs: u64,

    /// Age after which a stored assertion is refreshed; if the peer does not
    /// answer, it is served as stale until it expires (FED-17)
    pub assertion_refresh_secs: u64,

    /// Timeout of one call to a peer (FED-13)
    pub peer_timeout_ms: u64,

    /// Deadline for resolving a profile across peers (FED-13)
    pub resolve_deadline_ms: u64,

    /// Consecutive failures that open a peer's circuit (FED-21)
    pub max_failures_before_open: u32,

    /// Lifetime of a single-use link initiation URL (FED-42)
    pub link_request_ttl_secs: u64,

    /// Accepted `iat`/`exp` skew of a signed peer request (D-9)
    pub request_tolerance_secs: u64,
}

/// Raw config, validated into `FederationConfig`
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FederationConfigRaw {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default = "default_assertion_ttl_secs")]
    assertion_ttl_secs: u64,
    #[serde(default = "default_assertion_refresh_secs")]
    assertion_refresh_secs: u64,
    #[serde(default = "default_peer_timeout_ms")]
    peer_timeout_ms: u64,
    #[serde(default = "default_resolve_deadline_ms")]
    resolve_deadline_ms: u64,
    #[serde(default = "default_max_failures_before_open")]
    max_failures_before_open: u32,
    #[serde(default = "default_link_request_ttl_secs")]
    link_request_ttl_secs: u64,
    #[serde(default = "default_request_tolerance_secs")]
    request_tolerance_secs: u64,
}

fn default_assertion_ttl_secs() -> u64 {
    60
}

fn default_assertion_refresh_secs() -> u64 {
    30
}

fn default_peer_timeout_ms() -> u64 {
    800
}

fn default_resolve_deadline_ms() -> u64 {
    1500
}

fn default_max_failures_before_open() -> u32 {
    5
}

fn default_link_request_ttl_secs() -> u64 {
    600
}

fn default_request_tolerance_secs() -> u64 {
    30
}

impl TryFrom<FederationConfigRaw> for FederationConfig {
    type Error = String;

    fn try_from(raw: FederationConfigRaw) -> Result<Self, Self::Error> {
        check_refresh_age(&raw)?;

        let base_url = raw
            .base_url
            .ok_or_else(|| "core.federation.baseUrl is required".to_string())?;

        if !is_absolute_http_url(&base_url) {
            return Err(format!(
                "core.federation.baseUrl must be an absolute http(s) URL, \
                 got {base_url:?}"
            ));
        }

        Ok(Self {
            display_name: raw.display_name,
            base_url,
            assertion_ttl_secs: raw.assertion_ttl_secs,
            assertion_refresh_secs: raw.assertion_refresh_secs,
            peer_timeout_ms: raw.peer_timeout_ms,
            resolve_deadline_ms: raw.resolve_deadline_ms,
            max_failures_before_open: raw.max_failures_before_open,
            link_request_ttl_secs: raw.link_request_ttl_secs,
            request_tolerance_secs: raw.request_tolerance_secs,
        })
    }
}

/// A refresh age at or past the lifetime would never refresh before expiry
fn check_refresh_age(raw: &FederationConfigRaw) -> Result<(), String> {
    if raw.assertion_refresh_secs >= raw.assertion_ttl_secs {
        return Err(format!(
            "core.federation.assertionRefreshSecs ({}) must be below \
             assertionTtlSecs ({})",
            raw.assertion_refresh_secs, raw.assertion_ttl_secs
        ));
    }

    Ok(())
}

fn is_absolute_http_url(value: &str) -> bool {
    let Ok(uri) = value.parse::<Uri>() else {
        return false;
    };

    matches!(uri.scheme_str(), Some("http" | "https"))
        && uri.authority().is_some()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    core: TmpCoreConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpCoreConfig {
    #[serde(default)]
    federation: Option<FederationConfig>,
}

impl FederationConfig {
    /// Load `[core.federation]`, or `None` when the section is absent
    ///
    /// A present section is checked at boot: `baseUrl` must be absolute and
    /// `hmacPrimaryVersion` must not be 0, since a peer cannot be served
    /// without a signing key (FED-02).
    ///
    pub fn from_default_config_file(
        file: PathBuf,
        account_life_cycle: &AccountLifeCycle,
    ) -> Result<Option<Self>, MappedErrors> {
        let Some(config) =
            load_config_from_file::<TmpConfig>(file)?.core.federation
        else {
            return Ok(None);
        };

        check_signing_key_version(account_life_cycle.hmac_primary_version)?;

        Ok(Some(config))
    }
}

fn check_signing_key_version(primary: u32) -> Result<(), MappedErrors> {
    if primary == 0 {
        return creation_err(
            "core.accountLifeCycle.hmacPrimaryVersion must not be 0 while \
             [core.federation] is present: federation needs a signing key",
        )
        .as_error();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: serde_json::Value) -> Result<FederationConfig, String> {
        serde_json::from_value(value).map_err(|error| error.to_string())
    }

    #[test]
    fn a_section_with_only_the_base_url_takes_the_defaults() {
        let config =
            parse(serde_json::json!({"baseUrl": "https://myc-01.example.com"}))
                .unwrap();

        assert_eq!(
            config,
            FederationConfig {
                display_name: None,
                base_url: "https://myc-01.example.com".to_string(),
                assertion_ttl_secs: 60,
                assertion_refresh_secs: 30,
                peer_timeout_ms: 800,
                resolve_deadline_ms: 1500,
                max_failures_before_open: 5,
                link_request_ttl_secs: 600,
                request_tolerance_secs: 30,
            }
        );
    }

    #[test]
    fn a_missing_base_url_is_refused_by_name() {
        let error = parse(serde_json::json!({"displayName": "Instance 01"}))
            .unwrap_err();

        assert!(error.contains("baseUrl"), "{error}");
    }

    #[test]
    fn a_base_url_that_is_not_absolute_is_refused_by_name() {
        for url in ["myc-01.example.com", "/federation", "ftp://myc-01", ""] {
            let error = parse(serde_json::json!({"baseUrl": url})).unwrap_err();

            assert!(error.contains("baseUrl"), "{url}: {error}");
        }
    }

    #[test]
    fn a_refresh_age_not_below_the_lifetime_is_refused_by_name() {
        for (refresh, ttl) in [(60, 60), (90, 60)] {
            let error = parse(serde_json::json!({
                "baseUrl": "https://myc-01.example.com",
                "assertionTtlSecs": ttl,
                "assertionRefreshSecs": refresh,
            }))
            .unwrap_err();

            assert!(error.contains("assertionRefreshSecs"), "{error}");
        }
    }

    #[test]
    fn unknown_fields_are_refused() {
        let error = parse(serde_json::json!({
            "baseUrl": "https://myc-01.example.com",
            "signingKey": "nope",
        }))
        .unwrap_err();

        assert!(error.contains("signingKey"), "{error}");
    }

    #[test]
    fn primary_hmac_version_zero_is_refused_by_name() {
        let error = check_signing_key_version(0).unwrap_err().to_string();

        assert!(error.contains("hmacPrimaryVersion"), "{error}");
        assert!(check_signing_key_version(1).is_ok());
    }
}
