use async_trait::async_trait;
use chrono::Duration;
use clean_base::{entities::UpdatingResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait SessionTokenUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        session_key: String,
        time_to_live: Duration,
    ) -> Result<UpdatingResponseKind<bool>, MappedErrors>;
}
