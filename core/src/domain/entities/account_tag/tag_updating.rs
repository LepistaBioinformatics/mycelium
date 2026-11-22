use crate::domain::dtos::tag::Tag;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountTagUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        tag: Tag,
    ) -> Result<UpdatingResponseKind<Tag>, MappedErrors>;
}
