use async_trait::async_trait;
use myc_core::{
    domain::{
        dtos::tenant::{TenantMeta, TenantMetaKey},
        entities::{EncryptionKeyFetching, TelegramConfig},
        utils::{
            AAD_FIELD_TELEGRAM_BOT_TOKEN, AAD_FIELD_TELEGRAM_WEBHOOK_SECRET,
        },
    },
    models::AccountLifeCycle,
    use_cases::gateway::telegram::decrypt_telegram_secret,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use uuid::Uuid;

/// Resolve Telegram secrets from tenant meta at request time.
///
/// Values stored under `TelegramBotToken` and `TelegramWebhookSecret` are
/// AES-256-GCM ciphertexts written by `set_telegram_config`. They are
/// decrypted eagerly in `from_tenant_meta` so that the trait methods remain
/// synchronous from the caller's perspective.
pub struct TelegramConfigSvcRepo {
    bot_token: String,
    webhook_secret: String,
}

impl TelegramConfigSvcRepo {
    pub async fn from_tenant_meta(
        meta: &TenantMeta,
        tenant_id: Uuid,
        config: AccountLifeCycle,
        encryption_key_fetching_repo: &dyn EncryptionKeyFetching,
    ) -> Result<Self, MappedErrors> {
        let bot_token_enc = meta
            .get(&TenantMetaKey::TelegramBotToken)
            .cloned()
            .ok_or_else(|| {
                fetching_err("telegram_bot_token_not_configured")
                    .with_exp_true()
            })?;

        let webhook_secret_enc = meta
            .get(&TenantMetaKey::TelegramWebhookSecret)
            .cloned()
            .ok_or_else(|| {
                fetching_err("telegram_webhook_secret_not_configured")
                    .with_exp_true()
            })?;

        let bot_token = decrypt_telegram_secret(
            &bot_token_enc,
            tenant_id,
            config.clone(),
            encryption_key_fetching_repo,
            AAD_FIELD_TELEGRAM_BOT_TOKEN,
        )
        .await?;

        let webhook_secret = decrypt_telegram_secret(
            &webhook_secret_enc,
            tenant_id,
            config,
            encryption_key_fetching_repo,
            AAD_FIELD_TELEGRAM_WEBHOOK_SECRET,
        )
        .await?;

        Ok(Self {
            bot_token,
            webhook_secret,
        })
    }
}

#[async_trait]
impl TelegramConfig for TelegramConfigSvcRepo {
    async fn get_bot_token(
        &self,
        _tenant_id: Uuid,
    ) -> Result<String, MappedErrors> {
        Ok(self.bot_token.clone())
    }

    async fn get_webhook_secret(
        &self,
        _tenant_id: Uuid,
    ) -> Result<String, MappedErrors> {
        Ok(self.webhook_secret.clone())
    }
}
