use secrecy::SecretString;

// Re-export domain types from core so callers import from one place
pub use myc_core::domain::dtos::telegram::{
    TelegramUpdateId, TelegramUser, TelegramUserId,
};

// ? ---------------------------------------------------------------------------
// ? Raw initData wrapper
// ? ---------------------------------------------------------------------------

/// Raw URL-encoded initData string as received from the Telegram Mini App.
#[derive(Debug, Clone)]
pub struct InitData(pub String);

// ? ---------------------------------------------------------------------------
// ? Secret newtypes — zeroized on drop via SecretString
// ? ---------------------------------------------------------------------------

/// Telegram bot token. Wraps the raw string resolved from Vault.
/// Never logged, never serialized into responses.
pub struct BotToken(pub SecretString);

/// Webhook secret set via setWebhook. Wraps the raw string from Vault.
/// Used for constant-time comparison against X-Telegram-Bot-Api-Secret-Token.
pub struct WebhookSecret(pub SecretString);

// ? ---------------------------------------------------------------------------
// ? Error type
// ? ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum TelegramVerifyError {
    MissingHash,
    InvalidHmac,
    Expired,
    MissingUserField,
    MalformedUserField,
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn bot_token_debug_redacted() {
        let token = BotToken(SecretString::new("secret123".into()));
        let debug = format!("{:?}", token.0);
        assert!(!debug.contains("secret123"));
    }

    #[test]
    fn webhook_secret_debug_redacted() {
        let secret = WebhookSecret(SecretString::new("mysecret".into()));
        let debug = format!("{:?}", secret.0);
        assert!(!debug.contains("mysecret"));
    }

    #[test]
    fn bot_token_expose_works() {
        let token = BotToken(SecretString::new("secret123".into()));
        assert_eq!(token.0.expose_secret(), "secret123");
    }

    #[test]
    fn telegram_user_id_equality() {
        assert_eq!(TelegramUserId(42), TelegramUserId(42));
        assert_ne!(TelegramUserId(42), TelegramUserId(99));
    }
}
