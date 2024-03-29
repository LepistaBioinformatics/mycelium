use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait TagDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}

impl Display for dyn TagDeletion {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TagDeletion {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
