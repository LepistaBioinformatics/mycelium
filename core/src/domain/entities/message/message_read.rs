use crate::domain::dtos::message::{MessageSendingEvent, MessageStatus};

use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait LocalMessageReading: Interface + Send + Sync {
    async fn list_oldest_messages(
        &self,
        tail_size: i32,
        status: MessageStatus,
    ) -> Result<FetchManyResponseKind<MessageSendingEvent>, MappedErrors>;
}
