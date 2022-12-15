use crate::domain::dtos::message::MessageDTO;

use agrobase::{
    entities::default_response::CreateResponseKind, utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait MessageSending: Interface + Send + Sync {
    async fn send(
        &self,
        message: MessageDTO,
    ) -> Result<CreateResponseKind<MessageDTO>, MappedErrors>;
}
