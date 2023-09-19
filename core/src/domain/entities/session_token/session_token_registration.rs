use async_trait::async_trait;
use clean_base::{entities::CreateResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait SessionTokenRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        session_key: String,
        session_value: String,
    ) -> Result<CreateResponseKind<bool>, MappedErrors>;
}
