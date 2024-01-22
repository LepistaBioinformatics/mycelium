use crate::domain::dtos::webhook::WebHook;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait WebHookUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        webhook: WebHook,
    ) -> Result<UpdatingResponseKind<WebHook>, MappedErrors>;
}
