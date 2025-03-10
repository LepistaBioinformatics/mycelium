use crate::domain::dtos::message::{Message, MessageSendingEvent};

use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait LocalMessageWrite: Interface + Send + Sync {
    async fn send(
        &self,
        message_event: MessageSendingEvent,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors>;

    async fn update_message_event(
        &self,
        message_event: MessageSendingEvent,
    ) -> Result<(), MappedErrors>;

    async fn delete_message_event(&self, id: Uuid) -> Result<(), MappedErrors>;

    async fn ping(&self) -> Result<(), MappedErrors>;
}

#[async_trait]
pub trait RemoteMessageWrite: Interface + Send + Sync {
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors>;
}
