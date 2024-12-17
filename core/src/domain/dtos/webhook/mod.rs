mod responses;
mod trigger;

pub use responses::*;
pub use trigger::*;

use super::http_secret::HttpSecret;
use crate::models::AccountLifeCycle;

use chrono::{DateTime, Local};
use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebHook {
    /// The webhook id
    pub id: Option<Uuid>,

    /// The webhook name
    pub name: String,

    /// The webhook description
    pub description: Option<String>,

    /// The webhook url
    pub url: String,

    /// The webhook trigger
    pub trigger: WebHookTrigger,

    /// The webhook is active
    pub is_active: bool,

    /// The webhook created date
    pub created: DateTime<Local>,

    /// The webhook updated date
    pub updated: Option<DateTime<Local>>,

    /// The webhook secret
    ///
    /// Its important to note that the secret should be encrypted in the
    /// database and redacted on the response.
    ///
    secret: Option<HttpSecret>,
}

impl WebHook {
    pub fn new(
        name: String,
        description: Option<String>,
        url: String,
        trigger: WebHookTrigger,
        secret: Option<HttpSecret>,
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
        trigger: WebHookTrigger,
        secret: Option<HttpSecret>,
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

    pub fn redact_secret_token(&mut self) {
        if let Some(secret) = &mut self.secret {
            secret.redact_token();
        }
    }

    pub fn get_secret(&self) -> Option<HttpSecret> {
        self.secret.clone()
    }
}
