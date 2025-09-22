use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, ToSchema, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountMetaKey {
    /// Phone Number
    PhoneNumber,

    /// The Telegram User
    TelegramUser,

    /// The WhatsApp User
    WhatsAppUser,

    /// The account locale
    Locale,

    /// To specify any other meta key
    ///
    /// Specify any other meta key that is not listed here.
    Custom(String),
}

impl Display for AccountMetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountMetaKey::PhoneNumber => write!(f, "phone_number"),
            AccountMetaKey::TelegramUser => write!(f, "telegram_user"),
            AccountMetaKey::WhatsAppUser => write!(f, "whatsapp_user"),
            AccountMetaKey::Locale => write!(f, "locale"),
            AccountMetaKey::Custom(key) => write!(f, "custom:{}", key),
        }
    }
}

impl FromStr for AccountMetaKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("custom:") {
            return Ok(AccountMetaKey::Custom(s[7..].to_owned()));
        }

        match s {
            "phone_number" => Ok(AccountMetaKey::PhoneNumber),
            "telegram_user" => Ok(AccountMetaKey::TelegramUser),
            "whatsapp_user" => Ok(AccountMetaKey::WhatsAppUser),
            "locale" => Ok(AccountMetaKey::Locale),
            _ => Err(format!("Invalid key: {}", s)),
        }
    }
}

impl Serialize for AccountMetaKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            AccountMetaKey::Custom(key) => serializer
                .serialize_str(format!("custom:{key}", key = key).as_str()),
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}
