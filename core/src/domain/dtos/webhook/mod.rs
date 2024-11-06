mod responses;
mod secret;
mod trigger;

use mycelium_base::utils::errors::MappedErrors;
pub use responses::*;
pub use secret::*;
pub use trigger::*;

use crate::models::AccountLifeCycle;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebHook {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub trigger: WebhookTrigger,
    pub is_active: bool,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,

    secret: Option<WebHookSecret>,
}

impl WebHook {
    pub fn new(
        name: String,
        description: Option<String>,
        url: String,
        trigger: WebhookTrigger,
        secret: Option<WebHookSecret>,
    ) -> Self {
        Self {
            id: None,
            name,
            description,
            url,
            trigger,
            is_active: true,
            created: Local::now(),
            updated: None,
            secret,
        }
    }

    pub fn new_encrypted(
        name: String,
        description: Option<String>,
        url: String,
        trigger: WebhookTrigger,
        secret: Option<WebHookSecret>,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        let encrypted_secret = match secret {
            None => None,
            Some(secret) => Some(secret.encrypt_me(config)?),
        };

        Ok(Self {
            id: None,
            name,
            description,
            url,
            trigger,
            is_active: true,
            created: Local::now(),
            updated: None,
            secret: encrypted_secret,
        })
    }
}
