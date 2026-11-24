use crate::domain::dtos::webhook::{
    WebHookExecutionEvent, WebHookExecutionStatus, WebHookExecutionSummary,
    WebHookTrigger,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
#[cfg(test)]
use mockall::automock;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

/// Read-only view of webhook execution events for super users.
///
/// Implementations read the `webhook_execution_event` database view, which
/// exposes the dispatching state only: never the payload, the propagation
/// URLs or the response bodies.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait WebHookExecutionFetching: Interface + Send + Sync {
    /// Events newest first, optionally filtered by trigger and status.
    async fn list_events(
        &self,
        trigger: Option<WebHookTrigger>,
        status: Option<WebHookExecutionStatus>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<WebHookExecutionEvent>, MappedErrors>;

    /// Event counts grouped by trigger and status, optionally only for
    /// events created at or after `since`.
    async fn summarize_events(
        &self,
        since: Option<DateTime<Local>>,
    ) -> Result<FetchManyResponseKind<WebHookExecutionSummary>, MappedErrors>;
}
