use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Port for resolving per-tenant Telegram configuration secrets.
///
/// Values are resolved from Vault at call time — no caching. The raw secret
/// strings are returned so the port layer can wrap them in BotToken /
/// WebhookSecret newtypes without introducing a dependency on myc-http-tools
/// from core.
#[async_trait]
pub trait TelegramConfig: Interface + Send + Sync {
    /// Resolve the bot token for a tenant from Vault.
    ///
    /// Returns `Err` with `exp=true` if the tenant has no Telegram
    /// configuration (`TelegramBotTokenRef` absent in tenant.meta).
    async fn get_bot_token(
        &self,
        tenant_id: Uuid,
    ) -> Result<String, MappedErrors>;

    /// Resolve the webhook secret for a tenant from Vault.
    ///
    /// Returns `Err` with `exp=true` if not configured.
    async fn get_webhook_secret(
        &self,
        tenant_id: Uuid,
    ) -> Result<String, MappedErrors>;
}
