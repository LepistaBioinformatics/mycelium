use crate::domain::utils::derive_key_from_uuid;

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
    ///   — HMAC signing (no migration path; signatures get invalidated).
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
}
