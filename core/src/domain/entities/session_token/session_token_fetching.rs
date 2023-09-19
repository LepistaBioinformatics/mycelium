use async_trait::async_trait;
use clean_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait SessionTokenFetching: Interface + Send + Sync {
    async fn get(
        &self,
        session_key: String,
    ) -> Result<FetchResponseKind<String, Uuid>, MappedErrors>;
}
