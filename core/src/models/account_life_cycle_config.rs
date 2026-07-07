use crate::{
    domain::utils::derive_key_from_uuid,
    models::{HmacSecretEntry, HmacSecretSet},
};

use myc_config::secret_resolver::SecretResolver;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// This struct is used to manage the token secret and the token expiration
/// times.
///
/// This is not the final position of this struct, it will be moved to a
/// dedicated module in the future.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountLifeCycle {
    /// Domain name
    pub domain_name: SecretResolver<String>,

    /// Domain URL
    pub domain_url: Option<SecretResolver<String>>,

    /// Default language
    pub locale: Option<SecretResolver<String>>,

    /// Token expiration time in seconds
    ///
    /// This information is used to calculate the lifetime for new user
    /// registration
    pub token_expiration: SecretResolver<i64>,

    /// General Purpose email name
    pub noreply_name: Option<SecretResolver<String>>,

    /// General Purpose email
    pub noreply_email: SecretResolver<String>,

    /// Support email name
    pub support_name: Option<SecretResolver<String>>,

    /// Support email
    pub support_email: SecretResolver<String>,

    /// Token secret
    ///
    /// Toke secret is used to sign tokens
    pub(crate) token_secret: SecretResolver<String>,

    /// Version of the HMAC key used to sign every newly-issued
    /// connection string. Must be present in `hmac_secrets` — enforced by
    /// `validate_hmac_config` at startup.
    pub(crate) hmac_primary_version: u32,

    /// Versioned set of HMAC keys. All entries are available for
    /// verification; only the entry matching `hmac_primary_version` is
    /// used for signing.
    pub(crate) hmac_secrets: HmacSecretSet,
}

impl AccountLifeCycle {
    /// Derive the 256-bit KEK (Key Encryption Key) bytes from `token_secret`.
    ///
    /// **Derivation:** SHA-256(token_secret UUID bytes) → 32-byte KEK.
    /// This is the same derivation used by the legacy v1 encryption path, so
    /// existing v1 ciphertexts remain readable after the envelope migration.
    ///
    /// **Scope of the KEK:** it wraps two categories of DEK:
    /// - Per-tenant DEK — stored in `tenant.encrypted_dek` for each real
    ///   tenant; used for Telegram bot tokens and Telegram webhook secrets.
    /// - System DEK — stored in `tenant.encrypted_dek` for `id = UUID::nil`;
    ///   used for webhook HTTP secrets and all TOTP secrets (user, manager,
    ///   and staff — TOTP is user identity, never tenant-scoped).
    ///
    /// # `token_secret` is multi-purpose — rotation has side-effects
    ///
    /// Besides the KEK, the same `token_secret` is also consumed directly as
    /// an HMAC key in `UserAccountScope::sign_token` (connection-string
    /// signatures). **Rotating `token_secret` therefore has two simultaneous
    /// effects:**
    ///
    /// 1. All wrapped DEKs must be re-wrapped under the new KEK (handled by
    ///    the planned `myc-cli rotate-kek` command — see below).
    /// 2. **Every active connection-string signature becomes invalid.** There
    ///    is no re-signing path; issued connection strings that depend on the
    ///    old secret must be treated as revoked.
    ///
    /// Consumers to audit before rotating (grep for callers of
    /// `token_secret.async_get_or_error`):
    /// - `core/src/domain/dtos/http_secret.rs` — v1 legacy decrypt path.
    /// - `core/src/domain/dtos/user.rs` (`Totp::decrypt_me`) — v1 legacy
    ///   decrypt path.
    /// - `core/src/domain/dtos/token/connection_string/user_account_connection_string.rs`
    ///   — HMAC signing consumes `hmac_secrets[hmac_primary_version]`
    ///   (no fallback to `token_secret`). Rotating `token_secret`
    ///   therefore no longer invalidates connection strings; HMAC key
    ///   rotation is a separate, versioned procedure documented in
    ///   `docs/book/src/22-hmac-key-rotation.md`.
    ///
    /// # KEK rotation (not yet implemented — planned as `myc-cli rotate-kek`)
    ///
    /// 1. Update `token_secret` in config/Vault to a new UUID value.
    /// 2. Run `myc-cli rotate-kek` (TODO) — for every row in `tenant`
    ///    (including UUID::nil): `unwrap_dek(old_kek)` → `wrap_dek(new_kek)`
    ///    → update `tenant.encrypted_dek`. Data fields (`user.mfa`,
    ///    `tenant.meta`, `webhook.secret`) are never touched.
    /// 3. Restart all gateway instances to pick up the new config.
    /// 4. Accept that all previously-issued connection strings are now
    ///    invalid (see the multi-purpose note above).
    ///
    /// Note: `myc-cli migrate-dek` is for v1→v2 data migration only — it
    /// does NOT re-wrap DEKs and is not the right tool for KEK rotation.
    ///
    /// Rotation cost is O(number of tenants), not O(number of encrypted rows).
    #[tracing::instrument(name = "derive_kek_bytes", skip_all)]
    pub async fn derive_kek_bytes(&self) -> Result<[u8; 32], MappedErrors> {
        let token_secret = self.token_secret.async_get_or_error().await?;
        let key_uuid = Uuid::parse_str(&token_secret).map_err(|err| {
            dto_err(format!("failed_to_parse_token_secret_as_uuid: {err}"))
        })?;
        Ok(derive_key_from_uuid(&key_uuid))
    }

    /// Return a clone of this config with `token_secret` replaced by the
    /// supplied literal value.
    ///
    /// Used by `myc-cli rotate-kek` to construct the **old** KEK's
    /// resolver from an operator-supplied env var while keeping the
    /// **new** KEK's resolver in the normal config file. All other fields
    /// are unchanged.
    pub fn with_token_secret_override(&self, token_secret: String) -> Self {
        let mut clone = self.clone();
        clone.token_secret = SecretResolver::Value(token_secret);
        clone
    }

    /// Return a clone of this config with a single HMAC key (version 1)
    /// replaced by the supplied literal value, and `hmac_primary_version`
    /// set to match.
    ///
    /// Used by standalone mode to inject an autogenerated-and-persisted
    /// HMAC secret (SM-R9) in place of whatever placeholder the config file
    /// carries -- mirrors `with_token_secret_override`'s role for the KEK.
    pub fn with_hmac_secret_override(&self, hmac_secret: String) -> Self {
        let mut clone = self.clone();
        clone.hmac_primary_version = 1;
        clone.hmac_secrets = HmacSecretSet::new(vec![HmacSecretEntry {
            version: 1,
            secret: SecretResolver::Value(hmac_secret),
        }]);
        clone
    }

    /// Return the HMAC signing-key bytes for a specific version.
    ///
    /// Used by `verify_signature` to locate the key matching the `KVR`
    /// bean carried by the connection string — including tokens issued
    /// under a previous primary that has not yet been retired.
    #[tracing::instrument(name = "hmac_signing_key_for_version", skip_all)]
    pub(crate) async fn hmac_signing_key_for_version(
        &self,
        version: u32,
    ) -> Result<Vec<u8>, MappedErrors> {
        let Some(entry) = self.hmac_secrets.lookup(version) else {
            return dto_err(format!(
                "hmac_key_version_not_configured: {version}",
            ))
            .as_error();
        };

        let secret = entry.secret.async_get_or_error().await?;
        Ok(secret.into_bytes())
    }

    /// Return the current primary `(version, key_bytes)` used for
    /// signing. `sign_token` consumes this so the KVR bean and the HMAC
    /// key stay aligned.
    #[tracing::instrument(name = "hmac_primary_signing_key", skip_all)]
    pub(crate) async fn hmac_primary_signing_key(
        &self,
    ) -> Result<(u32, Vec<u8>), MappedErrors> {
        let version = self.hmac_primary_version;
        let key = self.hmac_signing_key_for_version(version).await?;
        Ok((version, key))
    }

    /// Check that `hmac_primary_version` is present in `hmac_secrets` and
    /// that the set is internally consistent.
    ///
    /// Invoked during config load; a failure aborts startup so the gateway
    /// never runs without a reachable primary HMAC key.
    pub fn validate_hmac_config(&self) -> Result<(), MappedErrors> {
        self.hmac_secrets.validate(self.hmac_primary_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("example.com".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: None,
            noreply_email: SecretResolver::Value(
                "noreply@example.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@example.com".to_string(),
            ),
            token_secret: SecretResolver::Value("placeholder".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("placeholder".to_string()),
            }]),
        }
    }

    #[tokio::test]
    async fn with_hmac_secret_override_replaces_the_primary_key(
    ) -> Result<(), MappedErrors> {
        let config = base_config()
            .with_hmac_secret_override("generated-hmac-secret".to_string());

        config.validate_hmac_config()?;

        let (version, key) = config.hmac_primary_signing_key().await?;
        assert_eq!(version, 1);
        assert_eq!(key, b"generated-hmac-secret".to_vec());

        Ok(())
    }
}
