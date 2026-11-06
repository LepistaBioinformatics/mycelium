use super::{preparation_limits::PreparationLimits, PreparedProfileSigning};
use myc_config::is_vault_configured;
use myc_core::{
    domain::dtos::native_error_codes::NativeErrorCodes,
    models::{ProfileSigningConfig, ProfileSigningKeyEntry},
};
use myc_http_tools::models::{
    Ed25519PublicJwk, ProfileSigningState, PublishedProfileKeys,
    VersionedEd25519Signer,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::time::timeout;

/// Upper bound for resolving one private key (env or Vault)
///
/// The Vault resolver uses an HTTP client without its own timeout; boot must
/// fail instead of hanging.
///
type KeyResolution<'a> =
    Pin<Box<dyn Future<Output = Result<String, MappedErrors>> + 'a>>;
type KeyResolver = for<'a> fn(&'a ProfileSigningKeyEntry) -> KeyResolution<'a>;

/// Prepare the downstream profile signing state once, at boot
///
/// - Section absent: signing disabled, legacy behavior preserved.
/// - Section present: every key (primary or not) is resolved, imported and
///   self-tested; any failure aborts boot. Only the primary signer is kept:
///   the gateway signs with the primary and never verifies. The public half
///   of every key is published as the profile JWKS.
///
/// Must run after Vault initialization and before the server accepts
/// traffic. Errors are sanitized: version and failing step, never material.
///
#[tracing::instrument(name = "prepare_profile_signing", skip_all)]
pub(crate) async fn prepare_profile_signing(
    config: Option<&ProfileSigningConfig>,
) -> Result<PreparedProfileSigning, MappedErrors> {
    let Some(config) = config else {
        tracing::info!(event = "profile_signature.startup", enabled = false);
        return Ok(PreparedProfileSigning::disabled());
    };

    prepare_enabled(config, resolve_key, PreparationLimits::default()).await
}

async fn prepare_enabled(
    config: &ProfileSigningConfig,
    resolver: KeyResolver,
    limits: PreparationLimits,
) -> Result<PreparedProfileSigning, MappedErrors> {
    config.validate()?;
    reject_vault_entries_without_vault(config)?;
    timeout(limits.total, prepare_keys(config, resolver, limits.per_key))
        .await
        .map_err(|_| {
            invalid(config.primary_key_version.value(), "preparation_timeout")
        })?
}

async fn prepare_keys(
    config: &ProfileSigningConfig,
    resolver: KeyResolver,
    per_key: Duration,
) -> Result<PreparedProfileSigning, MappedErrors> {
    let mut primary = None;
    let mut public_keys = Vec::with_capacity(config.keys.len());
    for entry in config.keys.iter() {
        let signer = prepare_entry(entry, resolver, per_key).await?;
        public_keys.push(signer.public_jwk().clone());
        let is_primary = entry.version == config.primary_key_version;
        primary = primary.or(is_primary.then_some(signer));
    }

    let signer = primary.ok_or_else(|| {
        invalid(config.primary_key_version.value(), "primary")
    })?;

    announce_enabled(config);
    Ok(enabled(signer, &public_keys, config))
}

fn enabled(
    signer: VersionedEd25519Signer,
    public_keys: &[Ed25519PublicJwk],
    config: &ProfileSigningConfig,
) -> PreparedProfileSigning {
    PreparedProfileSigning {
        state: ProfileSigningState::Enabled(Arc::new(signer)),
        published_keys: PublishedProfileKeys::new(
            public_keys,
            config.jwks_max_age_seconds,
        ),
    }
}

fn reject_vault_entries_without_vault(
    config: &ProfileSigningConfig,
) -> Result<(), MappedErrors> {
    if is_vault_configured() {
        return Ok(());
    }

    let Some(entry) = config.keys.iter().find(|entry| entry.uses_vault())
    else {
        return Ok(());
    };

    Err(invalid(entry.version.value(), "vault_not_configured"))
}

async fn prepare_entry(
    entry: &ProfileSigningKeyEntry,
    resolver: KeyResolver,
    per_key: Duration,
) -> Result<VersionedEd25519Signer, MappedErrors> {
    let version = entry.version.value();

    let private_key_pem = timeout(per_key, resolver(entry))
        .await
        .map_err(|_| invalid(version, "resolution_timeout"))?
        .map_err(|_| invalid(version, "resolution_failed"))?;

    VersionedEd25519Signer::from_pem(entry.version, &private_key_pem)
}

fn resolve_key(entry: &ProfileSigningKeyEntry) -> KeyResolution<'_> {
    Box::pin(entry.private_key.async_get_or_error())
}

fn announce_enabled(config: &ProfileSigningConfig) {
    tracing::info!(
        event = "profile_signature.startup",
        enabled = true,
        primary_key_version = config.primary_key_version.value(),
        key_versions = config.keys.len(),
        jwks_max_age_seconds = config.jwks_max_age_seconds.value(),
    );
}

fn invalid(version: u32, step: &str) -> MappedErrors {
    execution_err(format!(
        "profile_signing_key_invalid: version {version}, step {step}"
    ))
    .with_code(NativeErrorCodes::MYC00033)
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_config::secret_resolver::SecretResolver;
    use myc_core::{
        domain::dtos::profile_signature::{
            JwksMaxAgeSeconds, SigningKeyVersion,
        },
        models::ProfileSigningKeySet,
    };

    /// TEST ONLY: RFC 8032 test vector 1 (PKCS#8)
    const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";

    fn version(value: u32) -> SigningKeyVersion {
        SigningKeyVersion::try_from(value).unwrap()
    }

    fn entry(
        value: u32,
        key: SecretResolver<String>,
    ) -> ProfileSigningKeyEntry {
        ProfileSigningKeyEntry {
            version: version(value),
            private_key: key,
        }
    }

    fn config(
        primary: u32,
        entries: Vec<ProfileSigningKeyEntry>,
    ) -> ProfileSigningConfig {
        ProfileSigningConfig {
            primary_key_version: version(primary),
            keys: ProfileSigningKeySet::new(entries),
            jwks_max_age_seconds: JwksMaxAgeSeconds::default(),
        }
    }

    fn value(pem: &str) -> SecretResolver<String> {
        SecretResolver::Value(pem.to_string())
    }

    fn pending_key(_: &ProfileSigningKeyEntry) -> KeyResolution<'_> {
        Box::pin(std::future::pending())
    }

    fn failing_key(_: &ProfileSigningKeyEntry) -> KeyResolution<'_> {
        Box::pin(async { Err(execution_err("PRIVATE-TEST-MARKER")) })
    }

    fn unexpected_key(_: &ProfileSigningKeyEntry) -> KeyResolution<'_> {
        panic!("resolver must not run before the Vault guard")
    }

    fn limits(per_key_ms: u64, total_ms: u64) -> PreparationLimits {
        PreparationLimits {
            per_key: Duration::from_millis(per_key_ms),
            total: Duration::from_millis(total_ms),
        }
    }

    #[tokio::test]
    async fn resolution_timeout_is_sanitized() {
        let settings = config(1, vec![entry(1, value(PRIVATE_PEM))]);
        let error = prepare_enabled(&settings, pending_key, limits(5, 500))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("resolution_timeout"));
        assert!(error.contains("version 1"));
    }

    #[tokio::test]
    async fn total_budget_bounds_the_entire_preparation() {
        let settings = config(1, vec![entry(1, value(PRIVATE_PEM))]);
        let error = prepare_enabled(&settings, pending_key, limits(500, 5))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("preparation_timeout"));
    }

    #[tokio::test]
    async fn resolver_failure_never_exposes_the_underlying_error() {
        let settings = config(1, vec![entry(1, value(PRIVATE_PEM))]);
        let error = prepare_enabled(&settings, failing_key, limits(500, 1000))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("resolution_failed"));
        assert!(!error.contains("PRIVATE-TEST-MARKER"));
    }

    #[tokio::test]
    async fn vault_guard_precedes_the_resolver() {
        let settings = config(
            1,
            vec![entry(
                1,
                SecretResolver::Vault {
                    path: "test".into(),
                    key: "v1".into(),
                },
            )],
        );
        let error =
            prepare_enabled(&settings, unexpected_key, limits(500, 1000))
                .await
                .unwrap_err()
                .to_string();
        assert!(error.contains("vault_not_configured"));
    }

    #[test]
    fn vault_http_contract_isolated_process() {
        const FLAG: &str = "MYC_PSIG_VAULT_CHILD";
        if std::env::var_os(FLAG).is_none() {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "profile_signing::prepare_profile_signing::tests::vault_http_contract_isolated_process", "--nocapture"])
                .env(FLAG, "1").output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            return;
        }
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0; 4096];
            let length = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..length]);
            assert!(request.starts_with("GET /v1/test/data/signing HTTP/1.1"));
            assert!(request
                .to_lowercase()
                .contains("x-vault-token: test-only-token"));
            let body = serde_json::json!({"data":{"data":{"v1":PRIVATE_PEM}}})
                .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            myc_config::init_vault_config_from_file(
                None,
                Some(myc_config::optional_config::OptionalConfig::Enabled(
                    myc_config::VaultConfig {
                        url: format!("http://{address}"),
                        version_with_namespace: "v1/test".into(),
                        token: SecretResolver::Value("test-only-token".into()),
                    },
                )),
            )
            .await;
            let settings = config(
                1,
                vec![entry(
                    1,
                    SecretResolver::Vault {
                        path: "signing".into(),
                        key: "v1".into(),
                    },
                )],
            );
            assert!(prepare_profile_signing(Some(&settings))
                .await
                .unwrap()
                .is_enabled());
        });
        server.join().unwrap();
    }

    #[tokio::test]
    async fn absent_section_disables_signing() {
        let state = prepare_profile_signing(None).await.unwrap();
        assert!(!state.is_enabled());
    }

    #[tokio::test]
    async fn keeps_only_the_primary_signer() {
        let rotation = config(
            2,
            vec![entry(1, value(PRIVATE_PEM)), entry(2, value(PRIVATE_PEM))],
        );

        let prepared = prepare_profile_signing(Some(&rotation)).await.unwrap();
        let ProfileSigningState::Enabled(signer) = prepared.state else {
            panic!("signing should be enabled");
        };

        assert_eq!(signer.version().value(), 2);
    }

    #[tokio::test]
    async fn publishes_every_key_not_only_the_primary() {
        /// TEST ONLY: RFC 8032 test vector 2 (PKCS#8)
        const SECOND_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIEzNCJso/5banbbDRuwRTg9bijGfNaumJNqM9u1PuKb7
-----END PRIVATE KEY-----
";
        let rotation = config(
            2,
            vec![entry(1, value(PRIVATE_PEM)), entry(2, value(SECOND_PEM))],
        );

        let prepared = prepare_profile_signing(Some(&rotation)).await.unwrap();
        let PublishedProfileKeys::Enabled { body, max_age } =
            prepared.published_keys
        else {
            panic!("the key set should be published");
        };

        assert_eq!(max_age.value(), 43_200);
        assert!(body.contains("11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"));
        assert!(body.contains("PUAXw-hDiVqStwqnTRt-vJyYLM8uxJaMwM1V8Sr0Zgw"));
        assert!(!body.contains("\"d\""));
    }

    #[tokio::test]
    async fn absent_section_publishes_nothing() {
        let prepared = prepare_profile_signing(None).await.unwrap();

        assert!(matches!(
            prepared.published_keys,
            PublishedProfileKeys::Disabled
        ));
    }

    #[tokio::test]
    async fn any_invalid_entry_aborts_even_when_not_primary() {
        let rotation = config(
            2,
            vec![entry(1, value("broken")), entry(2, value(PRIVATE_PEM))],
        );

        let error = prepare_profile_signing(Some(&rotation)).await.unwrap_err();
        let message = error.to_string();

        assert!(message.contains("version 1"));
        assert!(!message.contains("broken"));
    }

    #[tokio::test]
    async fn structural_errors_abort() {
        let missing_primary = config(3, vec![entry(1, value(PRIVATE_PEM))]);
        let empty = config(1, vec![]);

        assert!(prepare_profile_signing(Some(&missing_primary))
            .await
            .is_err());
        assert!(prepare_profile_signing(Some(&empty)).await.is_err());
    }

    #[tokio::test]
    async fn missing_or_empty_env_aborts() {
        let unset = config(
            1,
            vec![entry(1, SecretResolver::Env("MYC_PSIG_TEST_UNSET".into()))],
        );

        unsafe { std::env::set_var("MYC_PSIG_TEST_EMPTY", "") };
        let empty = config(
            1,
            vec![entry(1, SecretResolver::Env("MYC_PSIG_TEST_EMPTY".into()))],
        );

        assert!(prepare_profile_signing(Some(&unset)).await.is_err());
        assert!(prepare_profile_signing(Some(&empty)).await.is_err());
    }

    #[tokio::test]
    async fn env_entry_with_multiline_pem_is_accepted() {
        unsafe { std::env::set_var("MYC_PSIG_TEST_PEM", PRIVATE_PEM) };
        let from_env = config(
            1,
            vec![entry(1, SecretResolver::Env("MYC_PSIG_TEST_PEM".into()))],
        );

        let state = prepare_profile_signing(Some(&from_env)).await.unwrap();
        assert!(state.is_enabled());
    }

    #[tokio::test]
    async fn vault_entry_without_vault_is_a_config_error_not_a_panic() {
        let vault = config(
            1,
            vec![entry(
                1,
                SecretResolver::Vault {
                    path: "myc/core/profileSigning".into(),
                    key: "v1".into(),
                },
            )],
        );

        let error = prepare_profile_signing(Some(&vault)).await.unwrap_err();
        assert!(error.to_string().contains("vault_not_configured"));
    }
}
