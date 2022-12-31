use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionManyResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait TokenCleanup: Interface + Send + Sync {
    async fn clean(
        &self,
        ignore_today: Option<bool>,
    ) -> Result<DeletionManyResponseKind<Vec<String>>, MappedErrors>;
}
