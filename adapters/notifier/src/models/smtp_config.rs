use super::SmtpSecurity;

use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use myc_config::{
    load_config_from_file, optional_config::OptionalConfig,
    secret_resolver::SecretResolver,
};
use mycelium_base::utils::errors::{creation_err, execution_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub host: SecretResolver<String>,
    pub username: SecretResolver<String>,
    pub password: SecretResolver<String>,
    pub port: SecretResolver<u16>,

    /// TLS mode for the connection. When omitted, it is auto-selected from
    /// `port` (587 -> STARTTLS, otherwise implicit TLS) so existing 465
    /// deployments are unaffected and 587 providers work without extra config.
    #[serde(default)]
    pub security: Option<SmtpSecurity>,
}

unsafe impl Send for SmtpConfig {}

// Loaded on its own -- not bundled with `QueueConfig` -- so full mode
// (where SMTP is required) doesn't force standalone's `[queue]`
// loader to also require a `[smtp]` section it has no use for.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    smtp: SmtpConfig,
}

// Standalone's `[smtp]` is opt-in -- defaults to `Disabled` when the section
// is absent, so standalone builds don't need a config change at all.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OptionalTmpConfig {
    #[serde(default)]
    smtp: OptionalConfig<SmtpConfig>,
}

impl SmtpConfig {
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
            Ok(config) => Ok(config.smtp),
            Err(err) => Err(err),
        }
    }

    /// Load `[smtp]` as optional -- used by the `standalone` build, where
    /// real SMTP is opt-in (SM-R8): if absent, `select_local_transport`
    /// falls through to file/stub instead.
    pub fn from_optional_config_file(
        file: PathBuf,
    ) -> Result<OptionalConfig<Self>, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<OptionalTmpConfig>(file) {
            Ok(config) => Ok(config.smtp),
            Err(err) => Err(err),
        }
    }

    /// Build a real `SmtpTransport` from this config. Shared by full mode's
    /// `NotifierClientImpl` and standalone's opt-in SMTP wiring (SM-R8), so
    /// the connection-building logic exists in exactly one place. The TLS mode
    /// is taken from `security`, falling back to a port-based default (#178).
    #[tracing::instrument(name = "build_smtp_transport", skip_all)]
    pub async fn build_transport(&self) -> Result<SmtpTransport, MappedErrors> {
        let host = self.host.async_get_or_error().await?;
        let username = self.username.async_get_or_error().await?;
        let password = self.password.async_get_or_error().await?;
        let port = self.port.async_get_or_error().await?;

        let security = self
            .security
            .unwrap_or_else(|| SmtpSecurity::from_port(port));

        let builder = match security {
            SmtpSecurity::StartTls => SmtpTransport::starttls_relay(&host),
            SmtpSecurity::Implicit => SmtpTransport::relay(&host),
        }
        .map_err(|err| {
            execution_err(format!("Failed to connect to SMTP: {err}"))
        })?;

        Ok(builder
            .credentials(Credentials::new(username, password))
            .port(port)
            .build())
    }
}
