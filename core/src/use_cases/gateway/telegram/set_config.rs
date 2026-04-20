use crate::{
    domain::{
        dtos::{profile::Profile, tenant::TenantMetaKey},
        entities::TenantRegistration,
        utils::{decrypt_string, encrypt_string},
    },
    models::AccountLifeCycle,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

/// Store Telegram bot token and webhook secret for a tenant.
///
/// Accepts plain-text secrets, encrypts each with AES-256-GCM
/// (`encrypt_string`), and stores the ciphertext under
/// `TelegramBotToken` / `TelegramWebhookSecret` in tenant meta.
///
/// The caller must hold tenant-ownership of `tenant_id`; ownership is
/// verified via `profile.with_tenant_ownership_or_error`.
///
/// **Key rotation**: if `AccountLifeCycle::token_secret` changes on the
/// server, this use case must be called again to re-encrypt with the new key.
#[tracing::instrument(
    name = "set_telegram_config",
    fields(tenant_id = %tenant_id),
    skip(bot_token, webhook_secret, config, tenant_registration)
)]
pub async fn set_telegram_config(
    profile: Profile,
    tenant_id: Uuid,
    bot_token: String,
    webhook_secret: String,
    config: AccountLifeCycle,
    tenant_registration: Box<&dyn TenantRegistration>,
) -> Result<(), MappedErrors> {
    profile.with_tenant_ownership_or_error(tenant_id)?;

    let encrypted_bot_token = encrypt_string(&bot_token, &config).await?;

    let encrypted_webhook_secret =
        encrypt_string(&webhook_secret, &config).await?;

    tenant_registration
        .register_tenant_meta(
            profile.get_owners_ids(),
            tenant_id,
            TenantMetaKey::TelegramBotToken,
            encrypted_bot_token,
        )
        .await
        .map_err(|e| use_case_err(format!("failed_to_store_bot_token: {e}")))?;

    tenant_registration
        .register_tenant_meta(
            profile.get_owners_ids(),
            tenant_id,
            TenantMetaKey::TelegramWebhookSecret,
            encrypted_webhook_secret,
        )
        .await
        .map_err(|e| {
            use_case_err(format!("failed_to_store_webhook_secret: {e}"))
        })?;

    Ok(())
}

/// Decrypt a Telegram secret stored via `set_telegram_config`.
///
/// Convenience wrapper used by adapter implementations that need to
/// recover the plain-text bot token or webhook secret from tenant meta.
#[tracing::instrument(name = "decrypt_telegram_secret", skip_all)]
pub async fn decrypt_telegram_secret(
    encrypted_b64: &str,
    config: AccountLifeCycle,
) -> Result<String, MappedErrors> {
    decrypt_string(encrypted_b64, &config).await
}
