use crate::domain::dtos::tag::Tag;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};

#[async_trait]
pub trait TagUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        tag: Tag,
    ) -> Result<UpdatingResponseKind<Tag>, MappedErrors>;
}

impl Display for dyn TagUpdating {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TagUpdating {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
