use myc_core::domain::{dtos::message::Message, entities::MessageSending};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::CreateResponseKind, utils::errors::MappedErrors,
};
use log::error;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = MessageSending)]
pub struct MessageSendingSqlDbRepository {}

#[async_trait]
impl MessageSending for MessageSendingSqlDbRepository {
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Message>, MappedErrors> {
        // TODO: to implements.
        error!("User message not send. Method already not implemented.");
        Ok(CreateResponseKind::Created(message))
    }
}
