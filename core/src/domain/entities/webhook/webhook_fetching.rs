use crate::domain::dtos::webhook::{
    WebHook, WebHookExecutionStatus, WebHookPayloadArtifact, WebHookTrigger,
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
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;

    /// List all webhooks by trigger
    ///
    /// WARNING: This method should only be used for internal purposes.
    ///
    async fn list_by_trigger(
        &self,
        trigger: WebHookTrigger,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;

    async fn fetch_execution_event(
        &self,
        max_events: u32,
        max_attempts: u32,
        status: Option<Vec<WebHookExecutionStatus>>,
    ) -> Result<FetchManyResponseKind<WebHookPayloadArtifact>, MappedErrors>;
}
