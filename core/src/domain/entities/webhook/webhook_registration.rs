use crate::domain::dtos::webhook::{
    WebHook, WebHookPropagationArtifact, WebHookTrigger,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait WebHookRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        webhook: WebHook,
    ) -> Result<CreateResponseKind<WebHook>, MappedErrors>;

    async fn register_execution_event(
        &self,
        correspondence_id: Uuid,
        trigger: WebHookTrigger,
        artifact: WebHookPropagationArtifact,
    ) -> Result<CreateResponseKind<Uuid>, MappedErrors>;
}
