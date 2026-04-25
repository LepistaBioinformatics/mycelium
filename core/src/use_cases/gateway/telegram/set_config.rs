use crate::{
    domain::{
        dtos::{profile::Profile, tenant::TenantMetaKey},
        entities::{EncryptionKeyFetching, TenantRegistration},
        utils::{
            build_aad, decrypt_string_with_dek, encrypt_with_dek,
            AAD_FIELD_TELEGRAM_BOT_TOKEN, AAD_FIELD_TELEGRAM_WEBHOOK_SECRET,
        },
    },
    models::AccountLifeCycle,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

/// Store Telegram bot token and webhook secret for a tenant.
///
/// Accepts plain-text secrets, encrypts each with the tenant DEK (v2 format),
/// and stores the ciphertext under `TelegramBotToken` / `TelegramWebhookSecret`
/// in tenant meta.
///
/// The caller must hold tenant-ownership of `tenant_id`; ownership is
/// verified via `profile.with_tenant_ownership_or_error`.
#[tracing::instrument(
    name = "set_telegram_config",
    fields(tenant_id = %tenant_id),
    skip(
        bot_token,
        webhook_secret,
        config,
        tenant_registration,
        encryption_key_fetching_repo
    )
)]
pub async fn set_telegram_config(
    profile: Profile,
    tenant_id: Uuid,
    bot_token: String,
    webhook_secret: String,
    config: AccountLifeCycle,
    tenant_registration: Box<&dyn TenantRegistration>,
    encryption_key_fetching_repo: Box<&dyn EncryptionKeyFetching>,
) -> Result<(), MappedErrors> {
    profile.with_tenant_ownership_or_error(tenant_id)?;

    let kek = config.derive_kek_bytes().await?;
    let dek = encryption_key_fetching_repo
        .get_or_provision_dek(Some(tenant_id), &kek)
        .await?;

    let bot_token_aad =
        build_aad(Some(tenant_id), AAD_FIELD_TELEGRAM_BOT_TOKEN);
    let webhook_aad =
        build_aad(Some(tenant_id), AAD_FIELD_TELEGRAM_WEBHOOK_SECRET);

    let encrypted_bot_token =
        encrypt_with_dek(&bot_token, &dek, &bot_token_aad)?;
    let encrypted_webhook_secret =
        encrypt_with_dek(&webhook_secret, &dek, &webhook_aad)?;

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
/// Accepts v1 (legacy) or v2 (DEK-based) ciphertext. v2 requires the tenant
/// DEK to be passed; v1 falls back to the config KEK.
#[tracing::instrument(name = "decrypt_telegram_secret", skip_all)]
pub async fn decrypt_telegram_secret(
    encrypted_b64: &str,
    tenant_id: Uuid,
    config: AccountLifeCycle,
    encryption_key_fetching_repo: &dyn EncryptionKeyFetching,
    field_aad: &[u8],
) -> Result<String, MappedErrors> {
    let kek = config.derive_kek_bytes().await?;
    let dek = encryption_key_fetching_repo
        .get_or_provision_dek(Some(tenant_id), &kek)
        .await?;
    let aad = build_aad(Some(tenant_id), field_aad);
    decrypt_string_with_dek(encrypted_b64, &config, &dek, &aad).await
}
