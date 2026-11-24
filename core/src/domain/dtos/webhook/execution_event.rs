use super::{WebHookExecutionStatus, WebHookTrigger};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// How the deliveries of one execution event went, without the target URLs
/// or response bodies (which can carry secrets or personal data).
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebHookPropagationSummary {
    /// Deliveries attempted to subscribed webhooks.
    pub total: u32,

    /// Deliveries answered with a non-2xx status.
    pub failed: u32,

    /// HTTP status of the most recent delivery, if any.
    pub last_response_status: Option<u16>,
}

/// One webhook execution event as super users see it: the dispatching
/// state, never the payload.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebHookExecutionEvent {
    pub id: Uuid,
    pub trigger: WebHookTrigger,
    pub payload_id: String,
    pub dispatch: WebHookDispatchState,
    pub propagations: WebHookPropagationSummary,
}

/// Where the dispatcher is with an event.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebHookDispatchState {
    pub status: WebHookExecutionStatus,
    pub attempts: u32,
    pub created: DateTime<Local>,

    /// When the last delivery attempt happened.
    pub attempted: Option<DateTime<Local>>,
}

/// Execution events grouped by trigger and status.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebHookExecutionSummary {
    pub trigger: WebHookTrigger,
    pub status: WebHookExecutionStatus,
    pub count: u64,

    /// Creation time of the newest event in the group.
    pub latest: DateTime<Local>,
}
