use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;
use uuid::Uuid;

use super::account::Account;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum HookTarget {
    Account,
    Guest,
}

impl Display for HookTarget {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            HookTarget::Account => write!(f, "account"),
            HookTarget::Guest => write!(f, "guest"),
        }
    }
}

impl FromStr for HookTarget {
    type Err = ();

    fn from_str(s: &str) -> Result<HookTarget, ()> {
        match s {
            "account" => Ok(HookTarget::Account),
            "guest" => Ok(HookTarget::Guest),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebHook {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub target: HookTarget,
    pub is_active: bool,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

impl WebHook {
    pub fn new(
        name: String,
        description: Option<String>,
        url: String,
        target: HookTarget,
    ) -> Self {
        Self {
            id: None,
            name,
            description,
            url,
            target,
            is_active: true,
            created: Local::now(),
            updated: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    pub url: String,
    pub status: u16,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountPropagationWebHookResponse {
    /// The account that was propagated.
    pub account: Account,

    /// Responses from the webhooks.
    pub propagation_responses: Option<Vec<HookResponse>>,
}
