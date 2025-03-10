use crate::domain::dtos::message::Message;

use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait LocalMessageSending: Interface + Send + Sync {
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors>;

    async fn ping(&self) -> Result<(), MappedErrors>;
}

#[async_trait]
pub trait RemoteMessageSending: Interface + Send + Sync {
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors>;
}
