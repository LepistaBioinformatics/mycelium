use async_trait::async_trait;
use chrono::{DateTime, Local};
use clean_base::{entities::CreateResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait SessionTokenRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        session_key: String,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<bool>, MappedErrors>;
}
