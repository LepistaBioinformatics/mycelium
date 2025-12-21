mod responses;
mod trigger;

pub use responses::*;
pub use trigger::*;

use super::http_secret::HttpSecret;
use crate::{
    domain::dtos::{http::HttpMethod, written_by::WrittenBy},
    models::AccountLifeCycle,
};

use chrono::{DateTime, Local};
use mycelium_base::utils::errors::{use_case_err, MappedErrors};
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

    /// The webhook method
    ///
    /// If the method is not provided, the default method is POST.
    /// Only write methods (POST, PUT, PATCH, DELETE) are allowed.
    ///
    #[serde(
        deserialize_with = "deserialize_write_method",
        default = "default_method",
        skip_serializing_if = "Option::is_none"
    )]
    pub method: Option<HttpMethod>,

    /// The webhook is active
    pub is_active: bool,

    /// The webhook created date
    pub created: DateTime<Local>,

    /// The webhook created by
    ///
    /// The ID of the account that created the webhook. This is used for
    /// auditing purposes.
    ///
    pub created_by: Option<WrittenBy>,

    /// The webhook updated date
    pub updated: Option<DateTime<Local>>,

    /// The webhook updated by
    ///
    /// The ID of the account that updated the webhook. This is used for
    /// auditing purposes.
    ///
    pub updated_by: Option<WrittenBy>,

    /// The webhook secret
    ///
    /// Its important to note that the secret should be encrypted in the
    /// database and redacted on the response.
    ///
    secret: Option<HttpSecret>,
}

fn default_method() -> Option<HttpMethod> {
    Some(HttpMethod::Post)
}

pub fn deserialize_write_method<'de, D>(
    deserializer: D,
) -> Result<Option<HttpMethod>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let method: Option<HttpMethod> = Option::deserialize(deserializer)?;

    if let Some(ref m) = method {
        if !WebHook::is_write_method(m) {
            return Err(serde::de::Error::custom(format!(
                "HTTP method '{method}' is not allowed. Only POST, PUT, PATCH and DELETE are allowed.",
                method = m
            )));
        }
    }

    Ok(method)
}

impl WebHook {
    pub fn new(
        name: String,
        description: Option<String>,
        url: String,
        trigger: WebHookTrigger,
        method: Option<HttpMethod>,
        secret: Option<HttpSecret>,
        created_by: Option<WrittenBy>,
    ) -> Self {
        if let Some(ref m) = method {
            if !WebHook::is_write_method(m) {
                panic!(
                    "HTTP method '{method}' is not allowed. Only POST, PUT, PATCH and DELETE are allowed.",
                    method = m
                );
            }
        }

        Self {
            id: None,
            name,
            description,
            url,
            trigger,
            method,
            is_active: true,
            created: Local::now(),
            created_by,
            updated: None,
            updated_by: None,
            secret,
        }
    }

    pub async fn new_encrypted(
        name: String,
        description: Option<String>,
        url: String,
        trigger: WebHookTrigger,
        method: Option<HttpMethod>,
        secret: Option<HttpSecret>,
        config: AccountLifeCycle,
        created_by: Option<WrittenBy>,
    ) -> Result<Self, MappedErrors> {
        if let Some(ref m) = method {
            if !WebHook::is_write_method(m) {
                return use_case_err(format!(
                    "HTTP method '{method}' is not allowed. Only POST, PUT, PATCH and DELETE are allowed.",
                    method = m
                ))
                .as_error();
            }
        }

        let encrypted_secret = match secret {
            None => None,
            Some(secret) => Some(secret.encrypt_me(config).await?),
        };

        Ok(Self {
            id: None,
            name,
            description,
            url,
            trigger,
            method,
            is_active: true,
            created: Local::now(),
            created_by,
            updated: None,
            updated_by: None,
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

    pub async fn set_secret(
        &mut self,
        secret: HttpSecret,
        config: AccountLifeCycle,
        updated_by: Option<WrittenBy>,
    ) -> Result<(), MappedErrors> {
        self.secret = Some(secret.encrypt_me(config).await?);
        self.updated_by = updated_by;
        Ok(())
    }

    pub fn is_write_method(method: &HttpMethod) -> bool {
        matches!(
            method,
            HttpMethod::Post
                | HttpMethod::Put
                | HttpMethod::Patch
                | HttpMethod::Delete
        )
    }
}
