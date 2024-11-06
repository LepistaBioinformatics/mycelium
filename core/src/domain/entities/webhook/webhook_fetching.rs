use crate::domain::dtos::webhook::{WebHook, WebhookTrigger};

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
        trigger: Option<WebhookTrigger>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;

    async fn list_by_trigger(
        &self,
        trigger: WebhookTrigger,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;
}
