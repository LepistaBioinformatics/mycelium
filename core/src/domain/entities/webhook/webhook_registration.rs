use crate::domain::dtos::webhook::WebHook;

use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait WebHookRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        webhook: WebHook,
    ) -> Result<CreateResponseKind<WebHook>, MappedErrors>;
}
