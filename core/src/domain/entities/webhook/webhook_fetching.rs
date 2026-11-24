use crate::domain::dtos::webhook::{
    WebHook, WebHookExecutionStatus, WebHookPayloadArtifact,
    WebHookRetryPolicy, WebHookTrigger,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait WebHookFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors>;

    async fn list(
        &self,
        name: Option<String>,
        trigger: Option<WebHookTrigger>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;

    /// List all webhooks by trigger
    ///
    /// WARNING: This method should only be used for internal purposes.
    ///
    async fn list_by_trigger(
        &self,
        trigger: WebHookTrigger,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;

    /// Claim a batch of execution events that are due for dispatch
    ///
    /// "Due" is not the same as "matching `status`": an event that failed
    /// recently is still serving its back-off and must not be handed out
    /// again, which is what `retry_policy` decides. Implementations that back
    /// a multi-pod deployment must also make the batch exclusive -- two
    /// replicas calling this concurrently may never receive the same event.
    ///
    async fn fetch_execution_event(
        &self,
        max_events: u32,
        max_attempts: u32,
        status: Option<Vec<WebHookExecutionStatus>>,
        retry_policy: WebHookRetryPolicy,
    ) -> Result<FetchManyResponseKind<WebHookPayloadArtifact>, MappedErrors>;
}
