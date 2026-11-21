use super::api_config::ApiConfig;

#[cfg(feature = "full")]
use myc_adapters_shared_lib::models::RedisConfig;
#[cfg(feature = "standalone")]
use myc_config::optional_config::OptionalConfig;
#[cfg(any(feature = "full", feature = "postgres-only"))]
use myc_config::{optional_config::OptionalConfig, VaultConfig};
use myc_core::models::CoreConfig;
#[cfg(feature = "federation")]
use myc_core::models::FederationConfig;
#[cfg(any(feature = "full", feature = "postgres-only"))]
use myc_diesel::models::config::DieselConfig;
#[cfg(feature = "standalone")]
use myc_diesel_sqlite::config::SqliteConfig;
use myc_http_tools::models::auth_config::AuthConfig;
#[cfg(feature = "standalone")]
use myc_notifier::models::LocalEmailConfig;
use myc_notifier::models::{QueueConfig, SmtpConfig};
use mycelium_base::utils::errors::MappedErrors;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ConfigHandler {
    pub core: CoreConfig,
    pub api: ApiConfig,
    pub auth: AuthConfig,
    // `QueueConfig` is just dispatcher polling behavior (queue name +
    // interval), not Redis-specific -- shared by both backends.
    pub queue: QueueConfig,

    #[cfg(any(feature = "full", feature = "postgres-only"))]
    pub diesel: DieselConfig,
    #[cfg(any(feature = "full", feature = "postgres-only"))]
    pub smtp: SmtpConfig,
    // Redis is full-mode only. `postgres-only` uses the `kv_artifact` Postgres
    // table for the KV cache, so `[redis]` is never read here.
    #[cfg(feature = "full")]
    pub redis: RedisConfig,
    #[cfg(any(feature = "full", feature = "postgres-only"))]
    pub vault: OptionalConfig<VaultConfig>,

    #[cfg(feature = "standalone")]
    pub sqlite: SqliteConfig,
    // Real SMTP is opt-in in standalone (SM-R8): absent by default, falls
    // through to file/stub transport (`select_local_transport`).
    #[cfg(feature = "standalone")]
    pub smtp: OptionalConfig<SmtpConfig>,
    // Opt-in local `.eml` file delivery (issue #169): absent by default, the
    // File rung of `select_local_transport`'s SMTP > File > Stub precedence.
    #[cfg(feature = "standalone")]
    pub local_email: OptionalConfig<LocalEmailConfig>,

    // `[core.federation]`: absent means the instance does not federate.
    #[cfg(feature = "federation")]
    pub federation: Option<FederationConfig>,
}

impl ConfigHandler {
    pub fn init_from_file(file: PathBuf) -> Result<Self, MappedErrors> {
        // Core configurations are used during the execution of the Mycelium
        // core functionalities, overall defined into use-cases layer.
        let core = CoreConfig::from_default_config_file(file.clone())?;

        Ok(Self {
            // Federation is checked against the HMAC set it derives its key
            // from, so it is loaded next to `core`.
            #[cfg(feature = "federation")]
            federation: FederationConfig::from_default_config_file(
                file.clone(),
                &core.account_life_cycle,
            )?,
            core,
            // API configuration should be used by the web server into the ports
            // layer.
            api: ApiConfig::from_default_config_file(file.clone())?,
            // Auth configuration should be used by the web server into the
            // ports.
            auth: AuthConfig::from_default_config_file(file.clone())?,
            // Queue configuration drives the email dispatcher's polling
            // behavior in both backends.
            queue: QueueConfig::from_default_config_file(file.clone())?,
            // Database configurations serves the database connector, which is
            // responsible for the communication with the database into the
            // adapters layer.
            #[cfg(any(feature = "full", feature = "postgres-only"))]
            diesel: DieselConfig::from_default_config_file(file.clone())?,
            // SMTP configuration should be used by the email sending repository
            // managements into the adapters layer.
            #[cfg(any(feature = "full", feature = "postgres-only"))]
            smtp: SmtpConfig::from_default_config_file(file.clone())?,
            // Redis configuration should be used by the redis repository
            // managements into the adapters layer. Full mode only --
            // `postgres-only` never reads `[redis]`.
            #[cfg(feature = "full")]
            redis: RedisConfig::from_default_config_file(file.clone())?,
            // Vault configuration should be used by the secret resolver into
            // the domain layer.
            #[cfg(any(feature = "full", feature = "postgres-only"))]
            vault: VaultConfig::from_default_config_file(file.clone())?,
            // SQLite configuration points at the standalone database file.
            #[cfg(feature = "standalone")]
            sqlite: SqliteConfig::from_default_config_file(file.clone())?,
            // Real SMTP is opt-in in standalone -- absent `[smtp]` resolves
            // to `OptionalConfig::Disabled`, not a load error.
            #[cfg(feature = "standalone")]
            smtp: SmtpConfig::from_optional_config_file(file.clone())?,
            // Opt-in local `.eml` file delivery -- absent `[localEmail]`
            // resolves to `Disabled`, falling through to the stub transport.
            #[cfg(feature = "standalone")]
            local_email: LocalEmailConfig::from_optional_config_file(
                file.clone(),
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "full")]
    #[test]
    fn config_full_example_toml_parses_into_config_handler() {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../settings/config.full.example.toml");

        ConfigHandler::init_from_file(file).unwrap();
    }

    #[cfg(feature = "postgres-only")]
    #[test]
    fn config_postgres_only_example_toml_parses_into_config_handler() {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../settings/config.postgres-only.example.toml");

        ConfigHandler::init_from_file(file).unwrap();
    }

    #[cfg(feature = "standalone")]
    #[test]
    fn config_standalone_example_toml_parses_into_config_handler() {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../settings/config.standalone.example.toml");

        let config = ConfigHandler::init_from_file(file).unwrap();

        // The example ships `[localEmail]` commented out -- absent section
        // must resolve to `Disabled`, not a load error.
        assert!(matches!(config.local_email, OptionalConfig::Disabled));
    }

    // End-to-end wiring for issue #169: a config with `[localEmail]` set
    // resolves through `ConfigHandler` and feeds `select_local_transport`,
    // which then selects the file transport (SMTP > File > Stub).
    #[cfg(feature = "standalone")]
    #[test]
    fn local_email_dir_wires_through_to_file_transport() {
        use myc_notifier::repositories::{
            select_local_transport, LocalTransportKind,
        };
        use std::io::Write;

        let base = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../settings/config.standalone.example.toml"),
        )
        .unwrap();

        let dir = std::env::temp_dir()
            .join(format!("myc_local_email_{}", std::process::id()));
        let with_local_email = format!(
            "{base}\n[localEmail.define]\ndir = {}\n",
            toml::Value::String(dir.to_string_lossy().into_owned())
        );

        let config_path = std::env::temp_dir().join(format!(
            "myc_config_local_email_{}.toml",
            std::process::id()
        ));
        let mut handle = std::fs::File::create(&config_path).unwrap();
        handle.write_all(with_local_email.as_bytes()).unwrap();

        let config =
            ConfigHandler::init_from_file(config_path.clone()).unwrap();

        let OptionalConfig::Enabled(local_email) = config.local_email else {
            panic!("[localEmail] should resolve to Enabled");
        };
        assert_eq!(local_email.dir, dir);

        // Mirror `main.rs`: the dir is created up-front (lettre's
        // `FileTransport` will not), then fed to `select_local_transport`.
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!dir.exists());
        let resolved = local_email.ensure_dir().unwrap();
        assert!(dir.is_dir());

        let selected = select_local_transport(None, Some(resolved));
        assert!(matches!(selected, LocalTransportKind::File(_)));

        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ? -----------------------------------------------------------------------
    // ? Federation examples
    // ? -----------------------------------------------------------------------

    #[cfg(feature = "federation")]
    #[test]
    fn shipped_example_does_not_federate() {
        let config =
            ConfigHandler::init_from_file(example_path(EXAMPLE.0)).unwrap();

        assert!(config.federation.is_none());
    }

    #[cfg(feature = "federation")]
    fn example_with_federation(edit: impl Fn(String) -> String) -> PathBuf {
        let content = std::fs::read_to_string(example_path(EXAMPLE.0)).unwrap();
        let content = edit(uncomment_from(&content, "#[core.federation]"));

        let path = std::env::temp_dir().join(format!(
            "myc-federation-{}-{}-{}",
            std::process::id(),
            content.len(),
            EXAMPLE.0
        ));
        std::fs::write(&path, content).unwrap();

        path
    }

    #[cfg(feature = "federation")]
    #[test]
    fn example_with_federation_uncommented_parses() {
        let path = example_with_federation(|content| content);
        let config = ConfigHandler::init_from_file(path.clone());
        let _ = std::fs::remove_file(&path);

        let federation = config.unwrap().federation.unwrap();

        assert_eq!(federation.base_url, "https://myc-01.example.com");
        assert_eq!(federation.display_name.as_deref(), Some("Instance 01"));
    }

    #[cfg(feature = "federation")]
    #[test]
    fn federation_refuses_boot_with_hmac_primary_version_zero() {
        // A valid version-0 entry, so only the federation check can refuse.
        let path = example_with_federation(|content| {
            let content = content
                .lines()
                .filter(|line| !line.starts_with("hmacPrimaryVersion"))
                .collect::<Vec<_>>()
                .join("\n")
                .replace(
                    "[core.accountLifeCycle]\n",
                    "[core.accountLifeCycle]\nhmacPrimaryVersion = 0\n",
                );

            format!(
                "{content}\n[[core.accountLifeCycle.hmacSecrets]]\n\
                 version = 0\nsecret = \"test-only-secret\"\n"
            )
        });
        let config = ConfigHandler::init_from_file(path.clone());
        let _ = std::fs::remove_file(&path);

        let error = config.err().expect("boot must be refused").to_string();
        assert!(error.contains("hmacPrimaryVersion"), "{error}");
    }

    // ? -----------------------------------------------------------------------
    // ? Downstream service notifications examples
    // ? -----------------------------------------------------------------------

    use crate::models::load_service_templates::load_service_templates;

    #[cfg(feature = "full")]
    const EXAMPLE: (&str, &str) =
        ("config.full.example.toml", "#[api.services]");

    #[cfg(feature = "postgres-only")]
    const EXAMPLE: (&str, &str) =
        ("config.postgres-only.example.toml", "#[api.services]");

    #[cfg(feature = "standalone")]
    const EXAMPLE: (&str, &str) = (
        "config.standalone.example.toml",
        "#[service-01.notifications]",
    );

    fn example_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../settings")
            .join(name)
    }

    /// Strip one leading `#` from the `anchor` line and from every commented
    /// line right after it, as an operator enabling the example would.
    fn uncomment_from(content: &str, anchor: &str) -> String {
        let mut in_block = false;

        content
            .lines()
            .map(|line| {
                in_block =
                    line == anchor || (in_block && line.starts_with('#'));

                match in_block {
                    true => &line[1..],
                    false => line,
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn shipped_example_boots_without_service_templates() {
        let config =
            ConfigHandler::init_from_file(example_path(EXAMPLE.0)).unwrap();

        let templates = load_service_templates(
            &config.api.services,
            &config.core.service_notifications,
        )
        .unwrap();

        assert!(templates.is_none());
        assert!(config.core.service_notifications.templates_dir.is_none());
    }

    #[test]
    fn example_with_notifications_uncommented_parses() {
        let (name, services_anchor) = EXAMPLE;
        let content = std::fs::read_to_string(example_path(name)).unwrap();
        let content = uncomment_from(&content, "#[core.serviceNotifications]");
        let content = uncomment_from(&content, services_anchor);

        let path = std::env::temp_dir()
            .join(format!("myc-notifications-{}-{name}", std::process::id()));
        std::fs::write(&path, content).unwrap();

        let config = ConfigHandler::init_from_file(path.clone());
        let _ = std::fs::remove_file(&path);
        let config = config.unwrap();

        let notifications = config
            .api
            .services
            .iter()
            .find_map(|service| service.notifications.as_ref())
            .expect("the example notifications block was not attached");

        assert_eq!(notifications.sender_tag, "service-01");
        assert_eq!(notifications.max_recipients_per_message, 10);
        assert_eq!(config.core.service_notifications.retention_days, 90);
        assert!(config.core.service_notifications.templates_dir.is_some());
    }
}
