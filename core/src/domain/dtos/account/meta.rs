use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum AccountMetaKey {
    /// Phone Number
    PhoneNumber,

    /// The Telegram User
    TelegramUser,

    /// The WhatsApp User
    WhatsAppUser,

    /// To specify any other meta key
    ///
    /// Specify any other meta key that is not listed here.
    #[serde(untagged)]
    Other(String),
}

impl Display for AccountMetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountMetaKey::PhoneNumber => write!(f, "PhoneNumber"),
            AccountMetaKey::TelegramUser => write!(f, "TelegramUser"),
            AccountMetaKey::WhatsAppUser => write!(f, "WhatsAppUser"),
            AccountMetaKey::Other(key) => write!(f, "{}", key),
        }
    }
}

impl FromStr for AccountMetaKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PhoneNumber" => Ok(AccountMetaKey::PhoneNumber),
            "TelegramUser" => Ok(AccountMetaKey::TelegramUser),
            "WhatsAppUser" => Ok(AccountMetaKey::WhatsAppUser),
            val => Ok(AccountMetaKey::Other(val.to_owned())),
        }
    }
}
